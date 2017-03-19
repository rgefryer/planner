use std::collections::HashMap;
use std::collections::BTreeMap;
use std::cell::RefCell;
use std::str::FromStr;
use std::rc::Rc;
use std::rc::Weak;
use std::fmt::Display;
use super::file::*;
use super::duration::*;
use super::time::*;
use super::timerow::*;
use super::SchedulingStrategy;
use super::ResourcingStrategy;
use super::web::*;

#[derive(Debug)]
struct ConfigNodeData {
	name: String,
	line_num: u32,
	indent: u32,
	level: u32,  // Root node is level 0
	attributes: HashMap<String, String>,

	// People are only defined on the root node
	people: HashMap<String, ChartTimeRow>,

	// Cells are only used on leaf nodes
	cells: ChartTimeRow,

    // Optional first and last dates that the task
    // can happen on.
    start: Option<ChartTime>,
    end: Option<ChartTime>,

	// Notes are problems to display on the chart
	notes: Vec<String>
}

impl ConfigNodeData {
    fn new(name: &str, level: u32, indent: u32, line_num: u32) 
            -> ConfigNodeData {
        ConfigNodeData { name: name.to_string(), 
                         line_num: line_num,
                         indent: indent, 
                         level: level, 
                         attributes: HashMap::new(), 
                         people: HashMap::new(),
                         cells: ChartTimeRow::new(),
                         start: None,
                         end: None,
                         notes: Vec::new() }

    }

    fn add_note(&mut self, note: &str) {

        self.notes.push(note.to_string());
    }

    fn update_start(&mut self, start: ChartTime) {

        // Move out the start date if necessary
        match self.start {
            Some(t) => {
                if start > t {
                    self.start = Some(start);
                }
            },
            None => {
                self.start = Some(start);
            }
        };
    }

    fn update_end(&mut self, end: ChartTime) {

        // Move in the end date if necessary
        match self.end {
            Some(t) => {
                if end < t {
                    self.end = Some(end);
                }
            },
            None => {
                self.end = Some(end);
            }
        };
    }
}

#[derive(Debug)]
pub struct ConfigNode {
    children: Vec<Rc<RefCell<ConfigNode>>>,

    parent: Option<Weak<RefCell<ConfigNode>>>, 

    data: RefCell<ConfigNodeData>,
}

impl ConfigNode {

	pub fn new(name: &str, level: u32, indent: u32, line_num: u32) 
			-> ConfigNode {
		ConfigNode { data: RefCell::new(ConfigNodeData::new(name, level, indent, line_num)),
					 children: Vec::new(),
					 parent: None }
	}

	fn create_attribute(&self, key: &str, val: &str) {
		self.data.borrow_mut().attributes.insert(key.to_string(), val.to_string());
	}

	fn new_child(&mut self, name: &str, indent: u32, line_num: u32) {
		self.children.push(Rc::new(RefCell::new(ConfigNode::new(name, 
																self.data.borrow().level+1,
																indent, 
																line_num))));
	}
		
	pub fn count_nodes(&self) -> u32 {
		let mut count = 1u32;

		for child_rc in &self.children {
			count += child_rc.borrow().count_nodes();
		}

		count
	}

	pub fn count_children(&self) -> u32 {
		let mut count = 0u32;

		for _ in &self.children {
			count += 1;
		}

		count
	}

	pub fn find_child_with_name(&self, name: &str) 
			-> Option<Weak<RefCell<ConfigNode>>> {

		if self.data.borrow().name == name {
			return None;
		} else {
			for child_rc in &self.children {
                let child_node = child_rc.borrow();
                if child_node.data.borrow().name == name {
                    return Some(Rc::downgrade(child_rc));
                } 
            }
        }

        None
    }

    pub fn get_weak_ref(&self) -> Option<Weak<RefCell<ConfigNode>>> {

        match self.parent {
            Some(ref p) => {
                let parent = p.upgrade().unwrap();
                for child_rc in &parent.borrow().children {
                    let child_node = child_rc.borrow();
                    if child_node.data.borrow().line_num == self.data.borrow().line_num {
                        return Some(Rc::downgrade(child_rc));
                    }
                }
                return None;
            },
            None => {
                return None;
            }
        };

    }

	pub fn get_inherited_attribute<T>(&self, key: &str) -> Result<Option<T>, String>
		where T: FromStr, 
		      <T as FromStr>::Err: Display {

		if self.data.borrow().attributes.contains_key(key) {
			return self.data.borrow().attributes[key].parse::<T>()
							.map_err(|e| format!("Problem parsing config {} on node at line {}: {}", 
												 key, 
												 self.data.borrow().line_num, 
												 e.to_string()))
							.map(|value| Some(value));
		} else if self.data.borrow().level == 1 {
			// There are no attributes on the root node.
			return Ok(None);
		} else {
			match self.parent {
				None => Ok(None),
				Some(ref p) => {
					match p.upgrade() {
						None => Ok(None),
						Some(node) => node.borrow()
										  .get_inherited_attribute(key)
					}
				}
			}
		}
	}


	/// Return true if this is a leaf node
	pub fn is_leaf(&self) -> bool {
		self.children.len() == 0
	}


	/// Get new hashmap containing all of the attributes
	pub fn get_attribute_hash(&self) -> HashMap<String, String> {
		let mut attr_hash = HashMap::new();

		for (key, val) in self.data.borrow().attributes.iter() {
			attr_hash.insert(key.clone(), val.clone());
		}

		attr_hash
	}

    /// Get a copy of the attributes on the [chart] node
    ///
    /// This must only be called on the root node.
	pub fn get_global_config(&self) -> HashMap<String, String> {
		match self.find_child_with_name("[chart]") {
			None => HashMap::new(),
			Some(node) => {
				match node.upgrade() {
					None => HashMap::new(),
					Some(n) => n.borrow().get_attribute_hash()
				}
			}
		}
	}

	/// Get a map from people to timerows.
	///
	/// It is an error if there are none defined, or if any are badly defined.
	pub fn get_people(&self, weeks: u32) 
			-> Result<HashMap<String, ChartTimeRow>, String> {
		
		let weak_node = try!(self.find_child_with_name("[people]")
				                 .ok_or("[people] node must exist"));
		let node_rc = weak_node.upgrade().unwrap();
        let node = node_rc.borrow();

		let mut people_hash = HashMap::new();
		for (key, val) in node.data.borrow().attributes.iter() {
			let ct = try!(ChartTimeRow::new_populate_range(val, weeks)
				.map_err(|e| format!("Problem setting up resource for {}: {}",
								     key, 
								     e.to_string())));
			people_hash.insert(key.clone(), ct);
		}
		Ok(people_hash)
	}

	/// Get a configuration value
    ///
    /// This must only be called on the root node.
	pub fn get_config_val<T>(&self, key: &str, default: Option<T>)
			-> Result<T, String> 
		where T: FromStr, 
		      <T as FromStr>::Err: Display {

		// Get global config
		let global_hash = self.get_global_config();

		// Read in resource information ([people])
		match global_hash.get(key) {
			Some(k) => {
				match k.parse::<T>() {
					Ok(v) => Ok(v),
					Err(e) => Err(format!("Problem parsing config {}: {}", 
										  key, e.to_string()))
				}
			},
			None => match default {
				Some(x) => Ok(x),
				None => Err(format!("No config in [chart] for {}", key))
			}
		}
	}

	pub fn transfer_local_committed_resource
			(&self, people_hash: &mut HashMap<String, ChartTimeRow>) 
			-> Result<(), String> {

		let valid_who: Vec<String> = people_hash.keys()
		                                        .map(|x| x.clone())
		                                        .collect();
		for (start, duration) in self.get_commitments() {
			let who: String;
			match self.get_who(&valid_who) {
				Ok(Some(w)) => {
					who = w;
				},
				Ok(None) => {
					self.add_note("Task has commitments but no owner");
					return Ok(());
				},
				Err(e) => {
					self.add_note(&e);
					continue;
				}
			}

            let mut node_data = self.data.borrow_mut();
			match people_hash.get_mut(&who)
			                 .unwrap()
			                 .fill_transfer_to
			                 	(&mut node_data.cells, 
								 duration.quarters() as u32, 
								 start.get_quarter() .. 
								   (start.get_quarter() + 
								    (duration.quarters() as u32))) {
				(_, _, 0) => {
					continue;
				},

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

	pub fn transfer_child_committed_resource
			(&self, people_hash: &mut HashMap<String, ChartTimeRow>) 
			-> Result<(), String> {

		// Now do any child nodes
		for child_rc in &self.children {
			try!(child_rc.borrow()
						 .transfer_local_committed_resource(people_hash));
			try!(child_rc.borrow()
						 .transfer_child_committed_resource(people_hash));
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

        match self.get_plan(&ChartTime::new(&format!("{}", weeks+1)).unwrap(), 
                                             &Duration::new_days(weeks as f32 * 5.0)) {
            Ok(Some(d)) => {
                days_in_plan = d;
            },
            Ok(None) => {
                return Ok(last_allocation);
			},
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
		let days_in_chart = Duration::new_quarters(self.data.borrow().cells.count() as i32);
		let days_to_allocate = days_in_plan - days_in_chart;
		if days_to_allocate.is_negative() {
			self.add_note(&format!("Over-committed by {} days; update plan", days_to_allocate.days() * -1.0));
			return Ok(last_allocation);
		}
		if days_to_allocate.is_zero() {
			return Ok(last_allocation);
		}

		// If there's no owner against this node, do nothing
		let valid_who: Vec<String> = people_hash.keys()
		                                        .map(|x| x.clone())
		                                        .collect();
		let who: String;
		match self.get_who(&valid_who) {
			Ok(Some(w)) => {
				who = w;
			},
			Ok(None) => {
				self.add_note("This task needs allocating to someone");
				return Ok(last_allocation);
			},
			Err(e) => {
				self.add_note(&e);
				return Ok(last_allocation);
			}
		};

        // Get start and end times as quarters
        let start_q = self.data.borrow().start.unwrap().get_quarter();
        let end_q = self.data.borrow().end.unwrap().get_quarter();

        // Get allocation type
        match self.get_resourcing_strategy() {
            Ok(Some(ResourcingStrategy::Management)) => {
                // No-op - the management row is handled out-of-band
            },
            Ok(Some(ResourcingStrategy::SmearProRata)) => {

                // Work out the time to spend per quarter day on this task.
                let quarters_in_plan = weeks * 20;
                let time_per_quarter = days_in_plan.quarters() as f32 / (quarters_in_plan as f32);

                // Work out the time to spend in the rest of the period
                let quarters_remaining = quarters_in_plan as i32 - start_q as i32;
                let mut time_to_spend = (quarters_remaining as f32 * time_per_quarter).ceil();

                // Subtract any time already committed.
                time_to_spend -= self.data.borrow().cells.count_range(start_q .. end_q) as f32;

                if time_to_spend < -0.01 {
                    self.add_note(&format!("Over-committed by {} days; update plan", time_to_spend * -1.0));
                } else {
                    // Smear the remainder.
                    let mut node_data = self.data.borrow_mut();
                    match people_hash.get_mut(&who)
                                     .unwrap()
                                     .smear_transfer_to(&mut node_data.cells,
                                                        time_to_spend as u32,
                                                        start_q .. end_q) {
                        (last, _, unallocated) if unallocated != 0 => {
                            node_data.add_note(&format!("{} days did not fit", unallocated as f32 / 4.0));
                            last_allocation = self.max_time(last_allocation, last);
                        },
                        (last, _, _) => {
                            last_allocation = self.max_time(last_allocation, last);
                        }
                    };
                }
            }
            Ok(Some(ResourcingStrategy::SmearRemaining)) => {
                // @@@ Implement it!
                // self.add_note(&"ResourcingStrategy::SmearRemaining not implemented!".to_string());
                let mut node_data = self.data.borrow_mut();
                match people_hash.get_mut(&who)
                                 .unwrap()
                                 .smear_transfer_to(&mut node_data.cells,
                                                    days_to_allocate.quarters() as u32,
                                                    start_q .. end_q) {
                    (last, _, unallocated) if unallocated != 0 => {
                        node_data.add_note(&format!("{} days did not fit", unallocated as f32 / 4.0));
                        last_allocation = self.max_time(last_allocation, last);
                    },
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
                                                   start_q .. end_q) {
                    (last, _, unallocated) if unallocated != 0 => {
                        node_data.add_note(&format!("{} days did not fit", unallocated as f32 / 4.0));
                        last_allocation = self.max_time(last_allocation, last);
                    },
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
            },
            Err(e) => {
                self.add_note(&format!("Unrecognised ResourcingStrategy: {}", e));
            },
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
            },
            Err(e) => {
                self.add_note(&e);
            },
            _ => {}
        };
        self.data.borrow_mut().update_start(earliest_ct);

        self.data.borrow_mut().update_end(
                ChartTime::new(&format!("{}", weeks+1)).unwrap());
        let mut latest_ct = self.data.borrow_mut().end.unwrap();
        match self.get_latest_end() {
            Ok(Some(ct)) => {
                latest_ct = ct;
            },
            Err(e) => {
                self.add_note(&e);
            },
            _ => {}
        };
        self.data.borrow_mut().update_end(latest_ct);

        // Do resource allocation on the local node.
        last_allocation = self.max_time_ct(last_allocation, 
            try!(self.allocate_node_task_resource(root, 
                                        managed, people_hash)));        

        // Work out whether to serialise the children.
        let mut scheduling_serial = false;
        match self.get_scheduling_strategy() {
            Ok(SchedulingStrategy::Serial) => {
                scheduling_serial = true;
            },
            Err(e) => {
                self.add_note(&e);
            },
            _ => {}
        };

        // Allocate resource in all the children
        for child_rc in &self.children {

            // If this node is marked as serial, then all child nodes
            // must start after any existing work is complete.
            let mut child_start_time: ChartTime = self.data.borrow().start.unwrap();
            if scheduling_serial {
                match last_allocation {
                    Some(ct) => {
                        child_start_time = ChartTime::new_from_quarter(ct.get_quarter() + 1);
                    },
                    None => {}
                };
            }

            // Allocate the child resource, update the last_allocation
            last_allocation = self.max_time_ct(last_allocation, try!(child_rc.borrow()
                                    .allocate_task_resource(root, 
                                        child_start_time, managed, people_hash)));
        }

        return Ok(last_allocation);
    }

    fn max_time(&self, a: Option<ChartTime>, b: Option<u32>) 
            -> Option<ChartTime> {

        match b {
            Some(b_q) => {
                let b_ct = ChartTime::new_from_quarter(b_q);
                match a {
                    Some(a_ct) => {
                        if b_ct > a_ct {
                            Some(b_ct)
                        } else {
                            Some(a_ct)
                        }
                    },
                    None => {
                        Some(b_ct)
                    }
                }
            },
            None => a
        }
    }

    fn max_time_ct(&self, a: Option<ChartTime>, b: Option<ChartTime>) 
            -> Option<ChartTime> {

        match b {
            Some(b_ct) => {
                match a {
                    Some(a_ct) => {
                        if b_ct > a_ct {
                            Some(b_ct)
                        } else {
                            Some(a_ct)
                        }
                    },
                    None => {
                        Some(b_ct)
                    }
                }
            },
            None => a
        }
    }

	/// Add a note to be displayed alongside this cell
	fn add_note(&self, note: &str) {

		self.data.borrow_mut().add_note(note);
	}

    fn get_management_row(&self)
            -> Option<Weak<RefCell<ConfigNode>>> {

        match self.get_resourcing_strategy() {
            Ok(Some(ResourcingStrategy::Management)) => {
                return self.get_weak_ref();
            },
            _ => {
                for child_rc in &self.children {
                    match child_rc.borrow().get_management_row() {
                        Some(node_ref) => {
                            return Some(node_ref);
                        },
                        None => {}
                    };
                }
            }
        };
        return None;
    }

    /// Allocate management resource
    fn allocate_management_resource(&self, 
                                    weeks: u32, 
                                    people_hash: &mut HashMap<String, ChartTimeRow>) 
            -> Result<(), String> {

        let node;
        match self.get_management_row() {
            Some(node_rc) => {
                node = node_rc.upgrade().unwrap();
            },
            None => {
                self.add_note("No management node, so no resource applied");
                return Ok(());            
            }
        };
        let management_row = node.borrow();

        let valid_who: Vec<String> = people_hash.keys()
                                                .map(|x| x.clone())
                                                .collect();
        let mut manager = String::new();
        let mut err_string = String::new();
        match management_row.get_who(&valid_who) {
            Ok(Some(who)) => {
                manager = who;
            },
            Ok(None) => {
                err_string = "No manager defined".to_string();
            },
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
        let start: ChartTime = 
                try!(self.get_config_val("today", Some(ChartTime::new("1").unwrap())));

        // Work out management resource on a quarter-day basis, 
        // add it up per week, then attempt to transfer to 
        // the management row.
        'week: for week in 1 .. weeks+1 {
            let mut weekly_resource = 0.0f32;

            'quarter: for q in (week-1)*20 .. week*20 {

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
                                               (week-1)*20 .. week*20) {
                (_, _, unallocated) if unallocated > 0 => {
                    management_data.add_note(&format!("Unable to allocate {} day(s) in week {}", unallocated as f32 / 4.0, week));
                },
                _ => {}
            };
        }
        Ok(())
    }

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

	/// Set up display data for this node and all children.
	pub fn display_gantt_internal(&self, 
								  root: &ConfigNode, 
								  context: &mut TemplateContext) 
			-> Result<(), String> {

		// Ignore "special" nodes
		for s in vec!["chart", "people", "rows"] {
			if self.data.borrow().name == format!("[{}]", s) {
				return Ok(());
			}
		}

	    let weeks: u32 = try!(root.get_config_val("weeks", None));

        // Get start time for the period to allocate.  Assume that everything
        // prior to this has been committed.
        let start: ChartTime = 
                try!(root.get_config_val("today", Some(ChartTime::new("1").unwrap())));
        //let end: ChartTime = ChartTime::new(&format!("{}", weeks+1)).unwrap();

        // Work out which is the start week.
        let start_week = start.get_quarter() / 20;

        // Set up row data for self
        let mut row = TemplateRow::new(self.data.borrow().level, self.data.borrow().line_num, &self.data.borrow().name);
        let mut count = 0;
        for val in &self.data.borrow().cells.get_weekly_numbers(weeks)  {
            row.add_cell(*val as f32 / 4.0, count == start_week);
            count += 1;
        }

        let done = self.data.borrow().cells.count_range(0 .. start.get_quarter()) as f32 / 4.0;
		row.set_done(done);

        let mut plan_now = Duration::new_days(0.0);
        match self.get_plan(&ChartTime::new(&format!("{}", weeks+1)).unwrap(), 
                                             &Duration::new_days(weeks as f32 * 5.0)) {
            Ok(Some(d)) => {
                plan_now = d;
                row.set_plan(d.days());
                row.set_left(d.days() - done);
            },
            Ok(None) => {}
            Err(e) => {
                self.add_note(&e);
            }
        }
        
        let mut plan_original = Duration::new_days(0.0);
        match self.get_plan(&ChartTime::new(&"1".to_string()).unwrap(), 
                                             &Duration::new_days(weeks as f32 * 5.0)) {
            Ok(Some(d)) => {
                plan_original = d;
            },
            Ok(None) => {}
            Err(e) => {
                self.add_note(&e);
            }
        }
        
        row.set_gain((plan_original - plan_now).days());

        for n in self.data.borrow().notes.iter() {
            row.add_note(n);
        }

		let valid_who: Vec<String> = root.data.borrow().people.keys()
                	                                          .map(|x| x.clone())
                	                                          .collect();

		match self.get_who(&valid_who) {
			Ok(Some(who)) => {
				row.set_who(&who);
			},
			Ok(None) => {},
			Err(e) => {
				self.add_note(&e);
			}
		};
        context.add_row(row);

	    // Set up row data for children
		for child_rc in &self.children {
	    	try!(child_rc.borrow().display_gantt_internal(root, context));
	    }
	    
	    Ok(())
	}

	/// Generate the data for displaying th gantt shart.
	///
	/// Sets up the resource rows, then recurses throught
	/// the node hierarchy.
	pub fn display_gantt(&self, context: &mut TemplateContext) 
			-> Result<(), String> {

	    let weeks: u32 = try!(self.get_config_val("weeks", None));

        // Get start time for the period to allocate.  Assume that everything
        // prior to this has been committed.
        let start: ChartTime = 
                try!(self.get_config_val("today", Some(ChartTime::new("1").unwrap())));

        //let end: ChartTime = ChartTime::new(&format!("{}", weeks+1)).unwrap();

        // Work out which is the start week.
        let start_week = start.get_quarter() / 20;

        // Set up row data for people
        for (who, cells) in &self.data.borrow().people {

            let mut row = TemplateRow::new(0, 0, &who);
            let mut count = 0;
            for val in &cells.get_weekly_numbers(weeks) {
				row.add_cell(*val as f32 / 4.0, count == start_week);
                count += 1;
			}
			row.set_left(cells.count() as f32 / 4.0);
	        context.add_row(row);
	    }

	    // Set up row data for nodes
	    try!(self.display_gantt_internal(self, context));

		//Err("display_gantt is not yet implemented".to_string())
	    Ok(())

	}

	// @@@ Derive Remaining and slip/gain

	fn augment_error(&self, err: String) -> String {
		format!("Problem in node at line {}: {}", 
				self.data.borrow().line_num, 
				err.to_string())

	}

	/// Get the non-managed status for the tasktask.
	///
	/// Non-managed status is inheritable, and defaults
	/// to false.     
	pub fn get_non_managed(&self) -> Result<bool, String> {
		match self.get_inherited_attribute("non-managed") {
			Ok(Some(val)) => Ok(val),
			Ok(None) => Ok(false),
			Err(e) => Err(self.augment_error(e))
		}
	}

	/// Get the latest end time for the task.
	///
	/// Latest end time is inheritable, and is
	/// not defaulted.
	pub fn get_latest_end(&self) -> Result<Option<ChartTime>, String> {
		match self.get_inherited_attribute::<String>("latest-end") {
			Ok(Some(ref time)) => {
				match ChartTime::new(time) {
					Ok(ct) => Ok(Some(ct)),
					Err(e) => Err(self.augment_error(e))
				}
			}
			Ok(None) => Ok(None),
			Err(e) => Err(self.augment_error(e))
		}
	}

	/// Get the earliest start time for the task.
	///
	/// Earliest start time is inheritable, and is
	/// not defaulted.
	pub fn get_earliest_start(&self) -> Result<Option<ChartTime>, String> {
		match self.get_inherited_attribute::<String>("earliest-start") {
			Ok(Some(ref time)) => {
				match ChartTime::new(time) {
					Ok(ct) => Ok(Some(ct)),
					Err(e) => Err(self.augment_error(e))
				}
			},
			Ok(None) => Ok(None),
			Err(e) => Err(self.augment_error(e))
		}
	}

	/// Get the resourcing commitments on this node
	///
	/// Commitments are not inheritable
	pub fn get_commitments(&self) -> Vec<(ChartTime, Duration)> {

		// Locate attributes that represent a commitment, and
		// build a map from ChartTime to duration.
		let mut map = BTreeMap::new();
		for (key, value) in &self.data.borrow().attributes {
			if key.starts_with('C') {
				match ChartTime::new(&key[1 ..]) {
					Ok(ct) => {
						match value.parse::<f32>() {
							Ok(d) => {
								map.insert(ct, Duration::new_days(d));
							},
							Err(_) => {
								continue;
							}
						}
					},
					Err(_) => {
						continue;
					}
				}
			}
		}

		// Convert the map to an ordered vector, and return it.
		let mut v = Vec::new();
		for (key, value) in map.iter() {
			v.push((key.clone(), value.clone()));
		}
		v
	}


	/// Get the strategy used to resource this node.
	///
	/// The resourcing strategy is inheritable, and
	/// there is no default.
	pub fn get_resourcing_strategy(&self) -> Result<Option<ResourcingStrategy>, String> {
		match self.get_inherited_attribute::<String>("resource") {
			Ok(Some(resource)) => {
				if resource == "management" {
					Ok(Some(ResourcingStrategy::Management))
				} else if resource == "smearprorata" {
					Ok(Some(ResourcingStrategy::SmearProRata))
				} else if resource == "smearremaining" {
					Ok(Some(ResourcingStrategy::SmearRemaining))
				} else if resource == "frontload" {
					Ok(Some(ResourcingStrategy::FrontLoad))
				} else if resource == "backload" {
					Ok(Some(ResourcingStrategy::BackLoad))
				} else if resource == "prodsfr" {
					Ok(Some(ResourcingStrategy::ProdSFR))
				} else {
					Err(self.augment_error(format!("Unrecognised resource, {}", resource)))
				}
			},

			Ok(None) => Ok(None),
			Err(e) => Err(self.augment_error(e))
		}
	}

	/// Get the approach used to schedule the child nodes
	///
	/// The scheduling approach is not inherited.  By default.
	/// a nodes children are scheduled in parallel
	pub fn get_scheduling_strategy(&self) -> Result<SchedulingStrategy, String> {
		
		let key = "schedule";
		if self.data.borrow().attributes.contains_key(key) {

			if self.data.borrow().attributes[key] == "parallel" {
				Ok(SchedulingStrategy::Parallel)
			} else if self.data.borrow().attributes[key] == "serial" {
				Ok(SchedulingStrategy::Serial)
			} else {
                Err(format!("Invalid scheduling strategy: {}", 
                    self.data.borrow().attributes[key]))
			}
		}
		else {
			Ok(SchedulingStrategy::Parallel)
		}
	}


    pub fn plan_string_to_dur(&self, 
                              plan_str: &String, 
                              when: &ChartTime, 
                              time_in_chart: &Duration) 
            -> Result<Option<Duration>, String> {

        // If plan_str contains multiple values, use the "when" time to select
        // the value that applies.
        let v: Vec<&str> = plan_str.split(", ").collect();
        let mut use_val: &str = "";
        let mut found = false;
        for val in v {
            let v2: Vec<&str> = val.split(":").collect();
            if v2.len() > 2 {
                return Err(self.augment_error(format!("Invalid plan part, {} has more \
                          than 2 parts", val)));
            }
            if v2.len() == 1 {
                found = true;
                use_val = val;
                continue;
            }
            match ChartTime::new(v2[0]) {
                Err(e) => {
                    return Err(self.augment_error(e));
                },
                Ok(ref ct) => {
                    if ct > when {
                        break
                    }
                    found = true;
                    use_val = v2[1];
                }
            }
        }

        if !found {
            return Ok(None);
        }

        // So, we have a value in use_val.  Try to convert it to a duration.
        match Duration::new_from_string(&use_val, time_in_chart) {
            Err(e) => {
                Err(self.augment_error(e))
            },
            Ok(dur) => {
                Ok(Some(dur))
            }
        }
    }

	/// Get the planned time for this task
	///
	/// The planned time cannot be inherited.  However, the default-plan
	/// can be, and applies to all leaf nodes that do not otherwise have
	/// a plan.
	///
	/// An example plan value is: "10, 2:11, 5.2:12, 8:11.25"
	///
	/// This would mean
	/// - The original plan was 10 days (this could also read 1:10)
	/// - In week 2, this was updated to 11 days
	/// - On Tuesday of week 5, this was updated to 12 days
	/// - In week 8, this was updated to 11.25 days
	///
	///	For this reason, the function requires a date, `when`, to
	/// specify the point for which the planned date is required.
	///
	///	The planned time, as well as being a number, can also be suffixed with
	/// pcy or pcm.  This function converts suffixed values into actual 
	/// durations.
	pub fn get_plan(&self, when: &ChartTime, time_in_chart: &Duration) 
			-> Result<Option<Duration>, String> {

        // Try to satisfy the request with local data
		let key = "plan";
		if self.data.borrow().attributes.contains_key(key) {
            match self.plan_string_to_dur(
                              &self.data.borrow().attributes[key], 
                              when, 
                              time_in_chart) {
                Ok(Some(d)) => {
                    return Ok(Some(d));
                },
                Ok(None) => {},
                Err(e) => {
                    return Err(e);
                }
            };
		}

        // Local data didn't cut it.  Next step is to look for a 
        // default we can inherit.  But first, bail out if this
        // is not a leaf node.
        if !self.is_leaf() {
            return Ok(None);
        }

        match self.get_inherited_attribute::<String>("default-plan") {
            Ok(Some(val)) => {
                return self.plan_string_to_dur(
                              &val, 
                              when, 
                              time_in_chart);
            },

            Ok(None) => { 
                return Ok(None);
            },

            Err(e) => {
                return Err(e);                  
            }
        };
	}

	/// Get the owner of this task
	///
	/// The owner can be inherited.  If this fails, the name of the
	/// node is returned as an owner.
	pub fn get_who(&self, valid: &Vec<String>) -> Result<Option<String>, String> {

		match self.get_inherited_attribute::<String>("who") {
			Ok(Some(who)) => {
				if valid.contains(&who) {
					return Ok(Some(who));
				} else {
					return Err(self.augment_error(format!("Unrecognised \"who\": {}", &who)));
				}
			},

			Ok(None) => {
				if valid.contains(&self.data.borrow().name) {
					return Ok(Some(self.data.borrow().name.clone()));
				}
				else {
					return Ok(None)
				}
			},

			Err(e) => {
				return Err(self.augment_error(e));
			}
		};
	}

	/// Get the budget, as a Duration.
	///
	/// The buget is not inherited - it is set against a single node, then
	/// compared with the plan/gain/commitments of all children.
	pub fn get_budget(&self) -> Option<Duration> {

		let key = "budget";
		if !self.data.borrow().attributes.contains_key(key) {
			return None;
		}

		match self.data.borrow().attributes[key].parse::<f32>() {
			Err(e) => {
				println!("Invalid budget in node at line {}: {}", 
						 self.data.borrow().line_num, e.to_string());
				return None;
			}
			Ok(dur) => {
				return Some(Duration::new_days(dur));
			}
		}
	}

	/// Return a weak reference to a child Node at a given line in the 
	/// config file.
	pub fn get_node_at_line(&self, line_num: u32) 
			-> Option<Weak<RefCell<ConfigNode>>> {

		if self.data.borrow().line_num == line_num {
			return None;
		}
		else {
			for child_rc in &self.children {
                let child = child_rc.borrow();
				if child.data.borrow().line_num == line_num {
					return Some(Rc::downgrade(child_rc));
				}
				else {
					match child_rc.borrow().get_node_at_line(line_num) {
						Some(x) => return Some(x),
						None => {}
					};
				}
			}
		}

		None
	}

	/// Read the config and build up a node hierarchy
	///
	/// This function is recursive
	/// - Child nodes are created, then given control
	/// - Higher-level nodes are passed to the parent to deal with
	pub fn consume_config(&mut self, 
						  ref_self: Option<&Rc<RefCell<ConfigNode>>>, 
						  file: &mut ConfigLines) 
			-> Result<(), String> {

		// Loop through the config, handling nodes and attributes differently
		loop {
			match file.peek_line() {
				Some(Line::Node(LineNode {line_num, indent, name } )) => {

					// Higher-level or sibling node - return to the parent
					// to handle.
					if self.data.borrow().indent >= indent {
						break;
					}

					// Create new child
					file.get_line().unwrap();
					self.new_child(&name, indent, line_num);

					// Set child's back-pointer as a weak reference to self
					let new_child = &self.children[self.children.len() - 1];
					new_child.borrow_mut().parent = match ref_self {
						None => None,
						Some(rc) => Some(Rc::downgrade(rc))
					};

					// Pass control to the child
					new_child.borrow_mut()
							 .consume_config(Some(&new_child), file).unwrap();
				}

				// Attributes are simply added to the current node.
				Some(Line::Attribute(LineAttribute {key, value } )) => {
					self.create_attribute(&key, &value);
					file.get_line().unwrap();
				}

				// End of config
				None => { break; }
			};
		}

		Ok(())
	}
}
	
