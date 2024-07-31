mod timetable;

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
        let mut tt = timetable::Timetable::new();
        if let Ok(_) = tt.gtfs_extract(&arg) {
            tt.deduplicate_stops();
            use spinoff::{spinners, Spinner};
            let mut spinner = Spinner::new(spinners::Dots, format!("Serializing"), None);
            if let Err(error) = tt.to_file("timetable.ron") {
                spinner.fail("Serialisation failed");
                eprintln!("while writing to file: {error}");
            } else {
                spinner.success("Done serialising");
            }
            tt.print_running_today();
        } else {
            use spinoff::{spinners, Spinner};
            let mut spinner = Spinner::new(spinners::Dots, format!("Reading file {arg}"), None);
            let mut file = std::fs::File::open(arg).unwrap();
            let mut buf = String::new();
            std::io::Read::read_to_string(&mut file, &mut buf).unwrap();
            spinner.success("Done reading");
            let mut spinner = Spinner::new(spinners::Dots, format!("Parsing..."), None);
            let mut tt: timetable::Timetable = ron::from_str(&buf).unwrap();
            spinner.success("Done parsing");
            tt.print_running_today();
            dbg!(tt.served_stops_today());
            // tt.deduplicate_stops();
            // dbg!(tt.served_stops_today());
        }
    }
}
