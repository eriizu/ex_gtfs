pub mod gtfs_extract;

use multimap::MultiMap;
use std::collections::HashMap;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Timetable {
    pub now: chrono::NaiveDateTime,
    pub today: chrono::NaiveDate,
    pub current_time: chrono::NaiveTime,
    pub calendar: HashMap<String, gtfs_structures::Calendar>,
    pub calendar_dates: HashMap<String, Vec<gtfs_structures::CalendarDate>>,
    pub stops: HashMap<String, gtfs_structures::Stop>,
    pub routes: HashMap<String, gtfs_structures::Route>,
    pub trips: MultiMap<String, Trip>,
}

#[derive(serde::Serialize, serde::Deserialize)]
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

#[derive(serde::Serialize, serde::Deserialize)]
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
        }
    }

    // TODO: function that takes a service id, gets cal and cal_date, checks
    // for day of the week and expetion.
    pub fn runs_on_weekday(&self, gtfs_cal: &gtfs_structures::Calendar) -> bool {
        let date = self.today;
        // dbg!(date);
        // dbg!(gtfs_cal);
        if date < gtfs_cal.start_date || date > gtfs_cal.end_date {
            return false;
        }
        let day_of_week = self.now.weekday();
        match day_of_week {
            chrono::Weekday::Mon if gtfs_cal.monday => true,
            chrono::Weekday::Tue if gtfs_cal.tuesday => true,
            chrono::Weekday::Wed if gtfs_cal.wednesday => true,
            chrono::Weekday::Thu if gtfs_cal.thursday => true,
            chrono::Weekday::Fri if gtfs_cal.friday => true,
            chrono::Weekday::Sat if gtfs_cal.saturday => true,
            chrono::Weekday::Sun if gtfs_cal.sunday => true,
            _ => false,
        }
    }

    pub fn to_file(&self, file_name_str: &str) -> Result<(), Box<dyn std::error::Error>> {
        let serialized = json5::to_string(&self)?;
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
}
