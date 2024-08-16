mod extractor;
mod timetable;

use chrono::prelude::*;
fn demo(tt: bus_model::TimeTable) {
    let now_naive: chrono::NaiveDateTime = {
        let now = Local::now();
        println!("{now:#?}");
        now.naive_local()
    };
    let now_date = now_naive.date();
    let mut journeys: Vec<_> = tt.get_journeys_for_day(&now_date).collect();
    journeys.sort_by(|a, b| a.stops[0].time.cmp(&b.stops[0].time));
    journeys
        .iter()
        .map(|journey| journey.stops[0].time)
        .for_each(|time| println!("{:02}:{:02}", time.hour(), time.minute()));
}

fn main() {
    let mut args = std::env::args();
    let Some(av1) = args.nth(1) else {
        return;
    };

    let now_naive: chrono::NaiveDateTime = {
        let now = Local::now();
        println!("{now:#?}");
        now.naive_local()
    };
    if av1 == "parse" {
        let mut tt = bus_model::TimeTable::new();
        use spinoff::{spinners, Spinner};
        let mut spinner = Spinner::new(spinners::Dots, "Parsing", None);
        let gtfs = gtfs_structures::Gtfs::new("../IDFM-gtfs.zip").unwrap();
        extractor::GtfsExtract::extract_gtfs_route(&mut tt, gtfs, "IDFM:C02298").unwrap();
        spinner.success("Done parsing");
        tt.get_journeys_for_day(&now_naive.date())
            .for_each(|journey| {
                dbg!(journey);
            });

        // TODO: this should be part of the model?
        let serialized =
            ron::ser::to_string_pretty(&tt, ron::ser::PrettyConfig::default()).unwrap();
        let mut file = std::fs::File::create("patate.ron").unwrap();
        std::io::Write::write(&mut file, serialized.as_bytes()).unwrap();
    } else if av1 == "read" {
        let file = std::fs::File::open("patate.ron").unwrap();
        let tt: bus_model::TimeTable = ron::de::from_reader(file).unwrap();
        demo(tt);
    }
    // gtfs_by_arg();

    // let now = std::time::SystemTime::now();
    // println!("{now:#?}");
    // let cc = Local::now();
    // println!("{cc:#?}");
    // let dd: chrono::NaiveDateTime = cc.naive_local();
}

fn gtfs_by_arg() {
    for arg in std::env::args().skip(1) {
        let mut tt = timetable::Timetable::new();
        if tt.gtfs_extract(&arg).is_ok() {
            tt.uniformise_stop_names();
            use spinoff::{spinners, Spinner};
            let mut spinner = Spinner::new(spinners::Dots, "Serializing", None);
            if let Err(error) = tt.to_file("timetable.ron") {
                spinner.fail("Serialisation failed");
                eprintln!("while writing to file: {error}");
            } else {
                spinner.success("Done serialising");
            }
            tt.print_running_today();
        } else {
            use spinoff::{spinners, Spinner};
            let mut spinner = Spinner::new(spinners::Dots, "Reading file {arg}", None);
            let mut file = std::fs::File::open(arg).unwrap();
            let mut buf = String::new();
            std::io::Read::read_to_string(&mut file, &mut buf).unwrap();
            spinner.success("Done reading");
            let mut spinner = Spinner::new(spinners::Dots, "Parsing...", None);
            let tt: timetable::Timetable = ron::from_str(&buf).unwrap();
            spinner.success("Done parsing");
            tt.print_running_today();
            dbg!(tt.served_stops_today());
            // tt.deduplicate_stops();
            // dbg!(tt.served_stops_today());
        }
    }
}
