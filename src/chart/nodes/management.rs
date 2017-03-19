use super::super::time::*;
use super::super::timerow::*;
use super::super::ResourcingStrategy;
use super::*;

impl ConfigNode {
    fn get_management_row(&self) -> Option<Weak<RefCell<ConfigNode>>> {

        match self.get_resourcing_strategy() {
            Ok(Some(ResourcingStrategy::Management)) => {
                return self.get_weak_ref();
            }
            _ => {
                for child_rc in &self.children {
                    match child_rc.borrow().get_management_row() {
                        Some(node_ref) => {
                            return Some(node_ref);
                        }
                        None => {}
                    };
                }
            }
        };
        return None;
    }

    /// Allocate management resource
    pub fn allocate_management_resource(&self,
                                        weeks: u32,
                                        people_hash: &mut HashMap<String, ChartTimeRow>)
                                        -> Result<(), String> {

        let node;
        match self.get_management_row() {
            Some(node_rc) => {
                node = node_rc.upgrade().unwrap();
            }
            None => {
                self.add_note("No management node, so no resource applied");
                return Ok(());
            }
        };
        let management_row = node.borrow();

        let valid_who: Vec<String> = people_hash.keys().map(|x| x.clone()).collect();
        let mut manager = String::new();
        let mut err_string = String::new();
        match management_row.get_who(&valid_who) {
            Ok(Some(who)) => {
                manager = who;
            }
            Ok(None) => {
                err_string = "No manager defined".to_string();
            }
            Err(e) => {
                err_string = format!("Invalid manager defined: {}", e);
            }
        }
        if err_string.len() != 0 {
            management_row.add_note(&err_string);
            return Ok(());
        }

        // Get start time for the period to allocate.  Assume that everything
        // prior to this has been committed.
        let start: ChartTime = try!(self.get_config_val("today",
                                                        Some(ChartTime::new("1").unwrap())));

        // Work out management resource on a quarter-day basis,
        // add it up per week, then attempt to transfer to
        // the management row.
        'week: for week in 1..weeks + 1 {
            let mut weekly_resource = 0.0f32;

            'quarter: for q in (week - 1) * 20..week * 20 {

                // Don't allocate historical resource
                if q < start.get_quarter() {
                    continue;
                }

                let mut quarterly_resource = 0.0f32;
                'person: for (who, row) in people_hash.iter() {
                    if *who == manager {
                        if !row.is_set(q) {
                            // The manager is not managing, so no
                            // management costs this quarter.
                            continue 'quarter;
                        }
                        // The manager doesn't have resource to
                        // manage himself.
                        continue 'person;
                    } else {
                        if row.is_set(q) {
                            // Add on 20% cost for managing the managee.
                            quarterly_resource += 0.2;
                        }
                    }
                }
                weekly_resource += quarterly_resource;
            }

            // Now transfer the weekly resource from the manager's
            // personal row to the management row.
            if weekly_resource > 20.0 {
                weekly_resource = 20.0;
            }
            let mut management_data = management_row.data.borrow_mut();
            match people_hash.get_mut(&manager)
                      .unwrap()
                      .fill_transfer_to(&mut management_data.cells,
                                        weekly_resource.ceil() as u32,
                                        (week - 1) * 20..week * 20) {
                (_, _, unallocated) if unallocated > 0 => {
                    management_data.add_note(&format!("Unable to allocate {} day(s) in week {}",
                                                      unallocated as f32 / 4.0,
                                                      week));
                }
                _ => {}
            };
        }
        Ok(())
    }
}
