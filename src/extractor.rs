pub trait GtfsExtract {
    fn extract_gtfs_route(
        &mut self,
        gtfs: gtfs_structures::Gtfs,
        route_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>>;
}

impl GtfsExtract for bus_model::TimeTable {
    fn extract_gtfs_route(
        &mut self,
        gtfs: gtfs_structures::Gtfs,
        route_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let journeys: Vec<_> = gtfs
            .trips
            .iter()
            .filter(|(_, candidate_trip)| candidate_trip.route_id == route_id)
            .map(|(_, value)| value)
            .filter_map(trip_convert)
            .collect();
        if journeys.is_empty() {
            return Err("no trip was available".into());
        }
        // let services: std::collections::HashSet<_> =
        journeys
            .iter()
            .map(|journey| journey.service_id.clone())
            .for_each(|service_id| extract_pattern_and_exceptions(self, &gtfs, service_id));
        // .collect();
        // for service_id in services {
        //     extract_pattern_and_exceptions(self, &gtfs, service_id);
        // }
        self.journeys = journeys;
        Ok(())
    }
}

fn extract_pattern_and_exceptions(
    tt: &mut bus_model::TimeTable,
    gtfs: &gtfs_structures::Gtfs,
    service_id: String,
) {
    if let Some(calendar) = gtfs.calendar.get(&service_id) {
        let pattern = callendar_to_pattern(calendar);
        tt.service_patterns.insert(service_id.clone(), pattern);
    }
    if let Some(callendar_dates) = gtfs.calendar_dates.get(&service_id) {
        for callendar_date in callendar_dates {
            let excpetion = bus_model::ServiceException {
                date: callendar_date.date,
                exception_type: match callendar_date.exception_type {
                    gtfs_structures::Exception::Added => bus_model::Exception::Added,
                    gtfs_structures::Exception::Deleted => bus_model::Exception::Deleted,
                },
            };
            tt.excpetions.insert(service_id.clone(), excpetion);
        }
    }
}

fn callendar_to_pattern(calendar: &gtfs_structures::Calendar) -> bus_model::ServicePattern {
    use bus_model::WeekdayFlags;
    let mut pattern = bus_model::ServicePattern {
        weekdays: WeekdayFlags::NEVER,
        start_date: calendar.start_date,
        end_date: calendar.end_date,
    };
    if calendar.monday {
        pattern.weekdays.set(WeekdayFlags::MONDAY, true);
    }
    if calendar.tuesday {
        pattern.weekdays.set(WeekdayFlags::TUESDAY, true);
    }
    if calendar.wednesday {
        pattern.weekdays.set(WeekdayFlags::WEDNESDAY, true);
    }
    if calendar.thursday {
        pattern.weekdays.set(WeekdayFlags::THURSDAY, true);
    }
    if calendar.friday {
        pattern.weekdays.set(WeekdayFlags::FRIDAY, true);
    }
    if calendar.saturday {
        pattern.weekdays.set(WeekdayFlags::SATURDAY, true);
    }
    if calendar.sunday {
        pattern.weekdays.set(WeekdayFlags::SUNDAY, true);
    }
    return pattern;
}

fn trip_convert(trip: &gtfs_structures::Trip) -> Option<bus_model::Journey> {
    let stops: Vec<_> = trip
        .stop_times
        .iter()
        .filter_map(stop_time_convert)
        .collect();

    if stops.is_empty() {
        None
    } else {
        Some(bus_model::Journey {
            service_id: trip.service_id.clone(),
            stops,
        })
    }
}

fn stop_time_convert(stop_time: &gtfs_structures::StopTime) -> Option<bus_model::StopTime> {
    let stop_name = stop_time.stop.name.clone()?;
    let seconds_from_midnight = stop_time.departure_time?;
    let time_of_day =
        chrono::NaiveTime::from_num_seconds_from_midnight_opt(seconds_from_midnight, 0)?;
    Some(bus_model::StopTime {
        time: time_of_day,
        stop_name: stop_name.clone(),
    })
}
