pub mod gtfs_extract;
pub mod runs_today;

use multimap::MultiMap;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

mod my_gtfs_structs {
    use structural_convert::StructuralConvert;

    #[derive(Clone, serde::Deserialize, serde::Serialize, Debug, StructuralConvert)]
    #[convert(from(gtfs_structures::Calendar))]
    pub struct Calendar {
        pub id: String,
        pub monday: bool,
        pub tuesday: bool,
        pub wednesday: bool,
        pub thursday: bool,
        pub friday: bool,
        pub saturday: bool,
        pub sunday: bool,
        pub start_date: chrono::NaiveDate,
        pub end_date: chrono::NaiveDate,
    }
    #[derive(Clone, serde::Deserialize, serde::Serialize, Debug, StructuralConvert)]
    #[convert(from(gtfs_structures::Stop))]
    pub struct Stop {
        pub id: String,
        pub code: Option<String>,
        pub name: Option<String>,
        pub description: Option<String>,
        // pub location_type: LocationType,
        pub parent_station: Option<String>,
        pub zone_id: Option<String>,
        pub url: Option<String>,
        pub longitude: Option<f64>,
        pub latitude: Option<f64>,
        pub timezone: Option<String>,
        // pub wheelchair_boarding: Availability,
        pub level_id: Option<String>,
        pub platform_code: Option<String>,
        // pub transfers: Vec<StopTransfer>,
        // pub pathways: Vec<Pathway>,
        pub tts_name: Option<String>,
    }

    #[derive(Clone, serde::Deserialize, serde::Serialize, Debug, StructuralConvert)]
    #[convert(from(gtfs_structures::Route))]
    pub struct Route {
        pub id: String,
        pub short_name: Option<String>,
        pub long_name: Option<String>,
        pub desc: Option<String>,
        // pub route_type: RouteType,
        pub url: Option<String>,
        pub agency_id: Option<String>,
        pub order: Option<u32>,
        // pub color: RGB8,
        // pub text_color: RGB8,
        // pub continuous_pickup: ContinuousPickupDropOff,
        // pub continuous_drop_off: ContinuousPickupDropOff,
    }

    #[derive(Clone, serde::Deserialize, serde::Serialize, Debug, StructuralConvert)]
    #[convert(from(gtfs_structures::CalendarDate))]
    pub struct CalendarDate {
        pub service_id: String,
        pub date: chrono::NaiveDate,
        pub exception_type: Exception,
    }

    #[derive(
        serde::Serialize,
        serde::Deserialize,
        Debug,
        PartialEq,
        Eq,
        Hash,
        Clone,
        Copy,
        StructuralConvert,
    )]
    #[convert(from(gtfs_structures::Exception))]
    pub enum Exception {
        Added,
        Deleted,
    }
    // impl std::convert::From<&gtfs_structures::Calendar> for Calendar {
    //     fn from(value: &gtfs_structures::Calendar) -> Self {
    //         Self {
    //             id: value.id.clone(),
    //             monday: value.monday,
    //             tuesday: value.tuesday,
    //             wednesday: value.wednesday,
    //             thursday: value.thursday,
    //             friday: value.friday,
    //             saturday: value.saturday,
    //             sunday: value.sunday,
    //             start_date: value.start_date,
    //             end_date: value.end_date,
    //         }
    //     }
    // }
}

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
        for trip in self
            .trips
            .iter()
            .map(|(_, b)| b)
            .filter(|trip| self.runs_today(&trip.service_id))
        {
            dbg!(trip);
        }
    }

    pub fn to_file(&self, file_name_str: &str) -> Result<(), Box<dyn std::error::Error>> {
        let serialized = ron::ser::to_string_pretty(&self, ron::ser::PrettyConfig::default())?;
        let first_size = serialized.len();
        // INFO: IDFM prefixes all IDs, even though without the prefix the IDs
        // do not collide. Striping them make data more concise and take up less
        // working memory and mass storage.
        // let serialized = serialized.replace("IDFM:TRANSDEV_MARNE_LA_VALLEE:", "");
        // let serialized = serialized.replace("IDFM:", "");
        // let second_size = serialized.len();
        // println!("\rserialized size: {second_size} bytes (before id simplification {first_size})");
        let mut file = std::fs::File::create(file_name_str)?;
        std::io::Write::write(&mut file, serialized.as_bytes())?;
        Ok(())
    }
}
