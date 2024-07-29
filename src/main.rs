mod model;
mod timetable;

use spinoff::{spinners, Spinner};
use timetable::Timetable;

fn gtfs_extract(arg: String) -> std::ops::ControlFlow<()> {
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
    let mut potator = Timetable::new();
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

fn main() {
    gtfs_by_arg();
    // let now = std::time::SystemTime::now();
    // println!("{now:#?}");
    // let cc = Local::now();
    // println!("{cc:#?}");
    // let dd: chrono::NaiveDateTime = cc.naive_local();
}

fn gtfs_by_arg() {
    for arg in std::env::args().skip(1) {
        if let std::ops::ControlFlow::Break(_) = gtfs_extract(arg) {
            return;
        }
    }
}
