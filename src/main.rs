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
        tt.gtfs_extract(&arg);
        use spinoff::{spinners, Spinner};
        let mut spinner = Spinner::new(spinners::Dots, format!("Serializing"), None);
        if let Err(error) = tt.to_file("timetable.json") {
            spinner.fail("Serialisation failed");
            eprintln!("while writing to file: {error}");
        } else {
            spinner.success("Done serialising");
        }
    }
}
