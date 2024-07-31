impl super::Timetable {
    // INFO: the dataset I tested this code with has an issues where individual
    // stops don't always have the same spelling. This functions goal is to make
    // the spelling uniform.
    pub fn uniformise_stop_names(&mut self) {
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
