pub mod gtfs_extract;
pub mod runs_today;

use multimap::MultiMap;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

mod my_gtfs_structs;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Timetable {
    pub now: chrono::NaiveDateTime,
    pub today: chrono::NaiveDate,
    pub current_time: chrono::NaiveTime,
    pub calendar: HashMap<String, my_gtfs_structs::Calendar>,
    pub calendar_dates: HashMap<String, Vec<my_gtfs_structs::CalendarDate>>,
    pub stops: HashMap<String, my_gtfs_structs::Stop>,
    pub routes: HashMap<String, my_gtfs_structs::Route>,
    pub trips: MultiMap<String, Trip>,
    running_services_cache: RefCell<HashSet<String>>,
    non_running_services_cache: RefCell<HashSet<String>>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Trip {
    pub id: String,
    pub service_id: String,
    pub route_id: String,
    pub stop_times: Vec<StopTime>,
}

impl From<&gtfs_structures::Trip> for Trip {
    fn from(value: &gtfs_structures::Trip) -> Self {
        Self {
            id: value.id.clone(),
            service_id: value.service_id.clone(),
            route_id: value.route_id.clone(),
            stop_times: value
                .stop_times
                .iter()
                .filter_map(|item| StopTime::try_from(item).ok())
                .collect(),
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct StopTime {
    pub time: chrono::NaiveTime,
    pub stop_id: String,
    pub name: String,
}

impl TryFrom<&gtfs_structures::StopTime> for StopTime {
    type Error = &'static str;
    fn try_from(value: &gtfs_structures::StopTime) -> Result<Self, Self::Error> {
        let time = chrono::NaiveTime::from_num_seconds_from_midnight_opt(
            value
                .departure_time
                .or(value.arrival_time)
                .ok_or("no arrival or departure time on stop")?,
            0,
        )
        .ok_or("could not convert arival/departure time to chrono::NaiveTime")?;
        Ok(Self {
            time,
            stop_id: value.stop.id.clone(),
            name: value.stop.name.clone().ok_or("stop without a name")?,
        })
    }
}

use chrono::prelude::*;

impl Timetable {
    pub fn new() -> Self {
        let now = Local::now();
        // println!("{now:#?}");
        let now_naive: chrono::NaiveDateTime = now.naive_local();
        Self {
            now: now_naive,
            today: now_naive.date(),
            current_time: now_naive.time(),
            calendar: HashMap::new(),
            calendar_dates: HashMap::new(),
            stops: HashMap::new(),
            routes: HashMap::new(),
            trips: MultiMap::new(),
            running_services_cache: RefCell::new(HashSet::new()),
            non_running_services_cache: RefCell::new(HashSet::new()),
        }
    }

    pub fn print_running_today(&self) {
        let mut trips: Vec<_> = self
            .trips
            .iter()
            .map(|(_, b)| b)
            .filter(|trip| self.runs_today(&trip.service_id))
            .collect();
        trips.sort_by(|a, b| {
            if let (Some(a_stop), Some(b_stop)) =
                (a.stop_times.iter().next(), b.stop_times.iter().next())
            {
                a_stop.time.cmp(&b_stop.time)
            } else {
                std::cmp::Ordering::Equal
            }
        });
        for trip in trips.iter() {
            // dbg!(trip);
            if let Some(first_stop_time) = trip.stop_times.iter().next() {
                println!("{}: {}", trip.id, first_stop_time.time);
            }
        }
    }

    pub fn served_stops_today(&self) -> Vec<String> {
        let mut set: HashSet<_> = self
            .trips
            .iter()
            .map(|(_, trip)| {
                trip.stop_times
                    .iter()
                    .map(|stop_time| stop_time.name.clone())
            })
            .flatten()
            .collect();
        let mut vector: Vec<_> = set.drain().collect();
        vector.sort();
        return vector;
    }

    pub fn to_file(&self, file_name_str: &str) -> Result<(), Box<dyn std::error::Error>> {
        let serialized = ron::ser::to_string_pretty(&self, ron::ser::PrettyConfig::default())?;
        let first_size = serialized.len();
        // INFO: IDFM prefixes all IDs, even though without the prefix the IDs
        // do not collide. Striping them make data more concise and take up less
        // working memory and mass storage.
        let serialized = serialized.replace("IDFM:TRANSDEV_MARNE_LA_VALLEE:", "");
        let serialized = serialized.replace("IDFM:", "");
        let second_size = serialized.len();
        println!("\rserialized size: {second_size} bytes (before id simplification {first_size})");
        let mut file = std::fs::File::create(file_name_str)?;
        std::io::Write::write(&mut file, serialized.as_bytes())?;
        Ok(())
    }

    // INFO: the dataset I tested this code with has an issues where individual
    // stops don't always have the same spelling. This functions goal is to make
    // the spelling uniform.
    pub fn deduplicate_stops(&mut self) {
        let iter = self.stops.iter().filter_map(|(id, stop)| match &stop.name {
            Some(name) => Some((id, name)),
            None => None,
        });
        let mut colliding: Vec<(String, String)> = vec![];
        for (id, name) in iter {
            let mut tmp_colliding: Vec<_> = self
                .stops
                .iter()
                .filter(|(filter_id, _)| *filter_id != id)
                .filter(|(filter_id, _)| {
                    colliding
                        .iter()
                        .find(|(a, b)| *a == **filter_id || *b == **filter_id)
                        .is_none()
                })
                .filter(|(_, filter_item)| {
                    if let Some(filter_name) = &filter_item.name {
                        if *filter_name != *name {
                            let lhs = unidecode::unidecode(filter_name).to_lowercase();
                            let rhs = unidecode::unidecode(name).to_lowercase();
                            if lhs == rhs {
                                println!("{filter_name} == {name}");
                                return true;
                            }
                        }
                    }
                    return false;
                })
                .map(|(fe_id, _)| (id.clone(), fe_id.clone()))
                .collect();
            tmp_colliding.drain(..).for_each(|a| colliding.push(a));
        }
        colliding
            .drain(..)
            .for_each(|ids| self.stop_uniformise_names(ids));
    }

    /// Replace name of the stop that is the shortest in bytes as it's most
    /// of the time the one that lacks the diacritics.
    fn stop_uniformise_names(&mut self, ids: (String, String)) {
        assert!(ids.0 != ids.1);
        let (Some(stop1), Some(stop2)) = (self.stops.get(&ids.0), self.stops.get(&ids.1)) else {
            return;
        };
        let (Some(name1), Some(name2)) = (&stop1.name, &stop2.name) else {
            return;
        };
        let (id, name) = if name1.len() > name2.len() {
            let name = name1.clone();
            (ids.1, name)
        } else {
            let name = name2.clone();
            (ids.0, name)
        };
        let stop = self.stops.get_mut(&id).expect("stop to still be there");
        stop.name = Some(name.clone());
        self.trips
            .iter_mut()
            .map(|(_, trip)| &mut trip.stop_times)
            .flatten()
            .filter(|stop_time| stop_time.stop_id == id)
            .for_each(|stop_time| stop_time.name = name.clone());
    }
}
