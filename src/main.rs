use spinoff::{spinners, Spinner};

fn gtfs_stuff() {
    for arg in std::env::args().skip(1) {
        if let std::ops::ControlFlow::Break(_) = gtfs_parse_and_explore(arg) {
            return;
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
    for (id, route) in gtfs.routes.iter() {
        println!("route id {id}");
        dbg!(route);
        for (id, trip) in gtfs
            .trips
            .iter()
            .filter(|(_, trip)| trip.route_id == route.id)
        {
            // dbg!(trip);
            for stop_time in &trip.stop_times {
                // INFO: time of day is stored in seconds
                let time_of_day = chrono::NaiveTime::from_num_seconds_from_midnight_opt(
                    stop_time.departure_time.unwrap(),
                    0,
                )
                .unwrap();
                if let Some(name) = &stop_time.stop.name {
                    println!("{id}: {name}: {time_of_day:#?}");
                }
            }
            return std::ops::ControlFlow::Break(());
        }
    }
    std::ops::ControlFlow::Continue(())
}

use chrono::prelude::*;
fn main() {
    gtfs_stuff();
    let now = std::time::SystemTime::now();
    println!("{now:#?}");
    let cc = Local::now();
    println!("{cc:#?}");
}
