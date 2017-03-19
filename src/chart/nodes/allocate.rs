use super::super::duration::*;
use super::super::time::*;
use super::super::timerow::*;
use super::super::SchedulingStrategy;
use super::super::ResourcingStrategy;
use super::*;

impl ConfigNode {
    /// Set up resource information in the chart
    ///
    /// This is only called on the root node.
    pub fn fill_in_gantt(&self) -> Result<(), String> {

        // Read in resource information ([people])
        let weeks: u32 = try!(self.get_config_val("weeks", None));
        let start_time = try!(self.get_config_val("today", Some(ChartTime::new("1").unwrap())));
        let mut people_hash = try!(self.get_people(weeks));

        // Move committed resource into the cells
        try!(self.transfer_child_committed_resource(&mut people_hash));

        // Handle all non-managed rows.  We'll then work out management
        // spend on the resource that hasn't yet been allocated.
        let managed = true;
        try!(self.allocate_task_resource(self, start_time, !managed, &mut people_hash));

        // Handle Management
        try!(self.allocate_management_resource(weeks, &mut people_hash));

        // Handle all managed rows
        try!(self.allocate_task_resource(self, start_time, managed, &mut people_hash));

        // Finally, store the people resources in the root_node
        self.data.borrow_mut().people = people_hash;

        Ok(())
    }

    pub fn transfer_local_committed_resource(&self,
                                             people_hash: &mut HashMap<String, ChartTimeRow>)
                                             -> Result<(), String> {

        let valid_who: Vec<String> = people_hash.keys().map(|x| x.clone()).collect();
        for (start, duration) in self.get_commitments() {
            let who: String;
            match self.get_who(&valid_who) {
                Ok(Some(w)) => {
                    who = w;
                }
                Ok(None) => {
                    self.add_note("Task has commitments but no owner");
                    return Ok(());
                }
                Err(e) => {
                    self.add_note(&e);
                    continue;
                }
            }

            let mut node_data = self.data.borrow_mut();
            match people_hash.get_mut(&who).unwrap().fill_transfer_to(&mut node_data.cells,
                                                                      duration.quarters() as u32,
                                                                      start.get_quarter()..
                                                                      (start.get_quarter() +
                                                                       (duration.quarters() as
                                                                        u32))) {
                (_, _, 0) => {
                    continue;
                }

                (_, ok, fail) => {
                    let mut err_string = String::new();
                    err_string.push_str(&"Unable to transfer resource".to_string());
                    err_string.push_str(&format!("\n  start={:?}", start));
                    err_string.push_str(&format!("\n  duration={:?}", duration));
                    err_string.push_str(&format!("\n  transferred={:?}", ok));
                    err_string.push_str(&format!("\n  missed={:?}", fail));
                    node_data.add_note(&err_string);
                    continue;
                }
            }
        }

        Ok(())
    }

    pub fn transfer_child_committed_resource(&self,
                                             people_hash: &mut HashMap<String, ChartTimeRow>)
                                             -> Result<(), String> {

        // Now do any child nodes
        for child_rc in &self.children {
            try!(child_rc.borrow().transfer_local_committed_resource(people_hash));
            try!(child_rc.borrow().transfer_child_committed_resource(people_hash));
        }

        Ok(())
    }



    /// Gantt out future resource for this node, and all children
    ///
    /// Returns the time of the last allocation, if there was one
    fn allocate_node_task_resource(&self,
                                   root: &ConfigNode,
                                   managed: bool,
                                   people_hash: &mut HashMap<String, ChartTimeRow>)
                                   -> Result<Option<ChartTime>, String> {

        // If there's no planned ressource against this node, do nothing.
        let mut last_allocation: Option<ChartTime> = None;
        let weeks: u32 = try!(root.get_config_val("weeks", None));
        let days_in_plan: Duration;

        // If this is not a leaf node, do nothing
        if !self.is_leaf() {
            return Ok(last_allocation);
        }

        // @@@ Duration passed into get_plan needs to be the time that
        // the user is in-plan.  This affects multiple calls.
        match self.get_plan(&ChartTime::new(&format!("{}", weeks + 1)).unwrap(),
                            &Duration::new_days(weeks as f32 * 5.0)) {
            Ok(Some(d)) => {
                days_in_plan = d;
            }
            Ok(None) => {
                return Ok(last_allocation);
            }
            Err(e) => {
                self.add_note(&e);
                return Ok(last_allocation);
            }
        };

        if days_in_plan.is_zero() {
            return Ok(last_allocation);
        }

        // If the managed state of this node doesn't match the "managed"
        // criteria, then there's nothing to be done.
        match self.get_non_managed() {
            Ok(true) => {
                if managed {
                    return Ok(last_allocation);
                }
            }
            Ok(false) => {
                if !managed {
                    return Ok(last_allocation);
                }
            }
            Err(e) => {
                self.add_note(&e);
                return Ok(last_allocation);
            }
        };

        // If there's no remaining work against this node, do nothing.
        let days_in_chart = Duration::new_quarters(self.data
                                                       .borrow()
                                                       .cells
                                                       .count() as
                                                   i32);
        let days_to_allocate = days_in_plan - days_in_chart;
        if days_to_allocate.is_negative() {
            self.add_note(&format!("Over-committed by {} days; update plan",
                                   days_to_allocate.days() * -1.0));
            return Ok(last_allocation);
        }
        if days_to_allocate.is_zero() {
            return Ok(last_allocation);
        }

        // If there's no owner against this node, do nothing
        let valid_who: Vec<String> = people_hash.keys().map(|x| x.clone()).collect();
        let who: String;
        match self.get_who(&valid_who) {
            Ok(Some(w)) => {
                who = w;
            }
            Ok(None) => {
                self.add_note("This task needs allocating to someone");
                return Ok(last_allocation);
            }
            Err(e) => {
                self.add_note(&e);
                return Ok(last_allocation);
            }
        };

        // Get start and end times as quarters
        // @@@ Both times should factor in the task-owners time-in-plan
        let start_q = self.data
            .borrow()
            .start
            .unwrap()
            .get_quarter();
        let end_q = self.data
            .borrow()
            .end
            .unwrap()
            .get_quarter() + 1;

        // Get allocation type
        match self.get_resourcing_strategy() {
            Ok(Some(ResourcingStrategy::Management)) => {
                // No-op - the management row is handled out-of-band
            }
            Ok(Some(ResourcingStrategy::SmearProRata)) => {

                // Work out the time to spend per quarter day on this task.
                // @@@ Should be time for this individual, not in full plan
                let quarters_in_plan = weeks * 20;
                let time_per_quarter = days_in_plan.quarters() as f32 / (quarters_in_plan as f32);

                // Work out the time to spend in the rest of the period
                let quarters_remaining = quarters_in_plan as i32 - start_q as i32;
                let mut time_to_spend = (quarters_remaining as f32 * time_per_quarter).ceil();

                // Subtract any time already committed.
                time_to_spend -= self.data
                    .borrow()
                    .cells
                    .count_range(start_q..end_q) as f32;

                if time_to_spend < -0.01 {
                    self.add_note(&format!("Over-committed by {} days; update plan",
                                           time_to_spend * -1.0));
                } else {
                    // Smear the remainder.
                    let mut node_data = self.data.borrow_mut();
                    match people_hash.get_mut(&who)
                              .unwrap()
                              .smear_transfer_to(&mut node_data.cells,
                                                 time_to_spend as u32,
                                                 start_q..end_q) {
                        (last, _, unallocated) if unallocated != 0 => {
                            node_data.add_note(&format!("{} days did not fit",
                                                        unallocated as f32 / 4.0));
                            last_allocation = self.max_time(last_allocation, last);
                        }
                        (last, _, _) => {
                            last_allocation = self.max_time(last_allocation, last);
                        }
                    };
                }
            }
            Ok(Some(ResourcingStrategy::SmearRemaining)) => {
                let mut node_data = self.data.borrow_mut();
                match people_hash.get_mut(&who)
                          .unwrap()
                          .smear_transfer_to(&mut node_data.cells,
                                             days_to_allocate.quarters() as u32,
                                             start_q..end_q) {
                    (last, _, unallocated) if unallocated != 0 => {
                        node_data.add_note(&format!("{} days did not fit",
                                                    unallocated as f32 / 4.0));
                        last_allocation = self.max_time(last_allocation, last);
                    }
                    (last, _, _) => {
                        last_allocation = self.max_time(last_allocation, last);
                    }
                };
            }
            Ok(Some(ResourcingStrategy::FrontLoad)) => {

                let mut node_data = self.data.borrow_mut();
                match people_hash.get_mut(&who)
                          .unwrap()
                          .fill_transfer_to(&mut node_data.cells,
                                            days_to_allocate.quarters() as u32,
                                            start_q..end_q) {
                    (last, _, unallocated) if unallocated != 0 => {
                        node_data.add_note(&format!("{} days did not fit",
                                                    unallocated as f32 / 4.0));
                        last_allocation = self.max_time(last_allocation, last);
                    }
                    (last, _, _) => {
                        last_allocation = self.max_time(last_allocation, last);
                    }
                };
            }
            Ok(Some(ResourcingStrategy::BackLoad)) => {
                // @@@ Implement it!
                self.add_note(&"ResourcingStrategy::BackLoad not implemented!".to_string());
            }
            Ok(Some(ResourcingStrategy::ProdSFR)) => {
                // @@@ Implement it!
                self.add_note(&"ResourcingStrategy::ProdSFR not implemented!".to_string());
            }
            Ok(None) => {
                self.add_note(&"This task needs a ResourcingStrategy".to_string());
            }
            Err(e) => {
                self.add_note(&format!("Unrecognised ResourcingStrategy: {}", e));
            }
        };

        return Ok(last_allocation);
    }

    /// Gantt out future resource for this node, and all children
    ///
    /// Returns the time of the last allocation, if there was one
    fn allocate_task_resource(&self,
                              root: &ConfigNode,
                              start_time: ChartTime,
                              managed: bool,
                              people_hash: &mut HashMap<String, ChartTimeRow>)
                              -> Result<Option<ChartTime>, String> {

        let mut last_allocation: Option<ChartTime> = None;
        let weeks: u32 = try!(root.get_config_val("weeks", None));

        // Ensure that a start and end time are set up for the allocation.
        self.data.borrow_mut().update_start(start_time);
        let mut earliest_ct = start_time;
        match self.get_earliest_start() {
            Ok(Some(ct)) => {
                earliest_ct = ct;
            }
            Err(e) => {
                self.add_note(&e);
            }
            _ => {}
        };
        self.data.borrow_mut().update_start(earliest_ct);

        self.data.borrow_mut().update_end(ChartTime::new(&format!("{}.5.4", weeks)).unwrap());
        let mut latest_ct = self.data
            .borrow_mut()
            .end
            .unwrap();
        match self.get_latest_end() {
            Ok(Some(ct)) => {
                latest_ct = ct;
            }
            Err(e) => {
                self.add_note(&e);
            }
            _ => {}
        };
        self.data.borrow_mut().update_end(latest_ct);

        // Do resource allocation on the local node.
        last_allocation =
            self.max_time_ct(last_allocation,
                             try!(self.allocate_node_task_resource(root, managed, people_hash)));

        // Work out whether to serialise the children.
        let mut scheduling_serial = false;
        match self.get_scheduling_strategy() {
            Ok(SchedulingStrategy::Serial) => {
                scheduling_serial = true;
            }
            Err(e) => {
                self.add_note(&e);
            }
            _ => {}
        };

        // Allocate resource in all the children
        for child_rc in &self.children {

            // If this node is marked as serial, then all child nodes
            // must start after any existing work is complete.
            let mut child_start_time: ChartTime = self.data
                .borrow()
                .start
                .unwrap();
            if scheduling_serial {
                match last_allocation {
                    Some(ct) => {
                        child_start_time = ChartTime::new_from_quarter(ct.get_quarter() + 1);
                    }
                    None => {}
                };
            }

            // Allocate the child resource, update the last_allocation
            last_allocation = self.max_time_ct(last_allocation,
                                               try!(child_rc.borrow()
                                    .allocate_task_resource(root,
                                        child_start_time, managed, people_hash)));
        }

        return Ok(last_allocation);
    }
}
