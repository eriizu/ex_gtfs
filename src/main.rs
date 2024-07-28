mod model;
use std::{collections::HashMap, sync::Arc};

use multimap::MultiMap;
use spinoff::{spinners, Spinner};

fn gtfs_stuff() {
    for arg in std::env::args().skip(1) {
        if let std::ops::ControlFlow::Break(_) = gtfs_parse_and_explore(arg) {
            return;
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Potator {
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
struct Trip {
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
struct StopTime {
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

impl Potator {
    fn new() -> Self {
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

    fn runs_on_weekday(&self, gtfs_cal: &gtfs_structures::Calendar) -> bool {
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
}

fn gtfs_parse_and_explore(arg: String) -> std::ops::ControlFlow<()> {
    let mut spinner = Spinner::new(spinners::Dots, format!("Parsing GTFS of: {arg}"), None);
    let gtfs = match gtfs_structures::Gtfs::new(&arg) {
        Ok(val) => val,
        Err(error) => {
            spinner.fail(&error.to_string());
            return std::ops::ControlFlow::Continue(());
        }
    };
    spinner.success("Parsing complete");
    println!("there are {} stops in the gtfs", gtfs.stops.len());
    let mut potator = Potator::new();
    for (id, route) in gtfs.routes.iter().filter(|route| route.0 == "IDFM:C02298") {
        potator.routes.insert(id.clone(), route.clone());
        println!("route id {id}");
        dbg!(route);
        for (id, trip) in gtfs
            .trips
            .iter()
            .filter(|(_, trip)| trip.route_id == route.id)
        {
            potator.trips.insert(id.clone(), trip.into());
            if let Some(service_cal) = gtfs.calendar.get(&trip.service_id) {
                potator
                    .calendar
                    .insert(trip.service_id.clone(), service_cal.clone());
                if !potator.runs_on_weekday(service_cal) {
                    continue;
                }
            }
            if let Some(service_date) = gtfs.calendar_dates.get(&trip.service_id) {
                potator
                    .calendar_dates
                    .insert(trip.service_id.clone(), service_date.clone());
            }
            for stop_time in &trip.stop_times {
                potator
                    .stops
                    .insert(stop_time.stop.id.clone(), (*stop_time.stop).clone());
                // INFO: time of day is stored in seconds
                // let time_of_day = chrono::NaiveTime::from_num_seconds_from_midnight_opt(
                //     stop_time.departure_time.unwrap(),
                //     0,
                // )
                // .unwrap();
                // if let Some(name) = &stop_time.stop.name {
                //     println!("{id}: {name}: {time_of_day:#?}");
                // }
                // break;
            }
            // return std::ops::ControlFlow::Continue(());
        }
    }
    let mut spinner = Spinner::new(spinners::Dots, format!("Serializing"), None);
    let serialized = json5::to_string(&potator).unwrap();
    let mut file = std::fs::File::create("potator.json").unwrap();
    std::io::Write::write(&mut file, serialized.as_bytes()).unwrap();
    spinner.success("Done serialising");
    std::ops::ControlFlow::Continue(())
}

use chrono::prelude::*;
fn main() {
    gtfs_stuff();
    // let now = std::time::SystemTime::now();
    // println!("{now:#?}");
    // let cc = Local::now();
    // println!("{cc:#?}");
    // let dd: chrono::NaiveDateTime = cc.naive_local();
}
