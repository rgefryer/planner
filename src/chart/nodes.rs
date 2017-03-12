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

#[derive(Debug)]
pub struct ConfigNode {
	name: String,
	line_num: u32,
	indent: u32,
	attributes: HashMap<String, String>,

	children: Vec<Rc<RefCell<ConfigNode>>>,

	parent: Option<Weak<RefCell<ConfigNode>>>, 

	// Cells are only used on leaf nodes
	cells: ChartTimeRow,
}

impl ConfigNode {

	pub fn new(name: &str, indent: u32, line_num: u32) -> ConfigNode {
		ConfigNode { name: name.to_string(), 
			         line_num: line_num,
					 indent: indent, 
					 attributes: HashMap::new(), 
					 children: Vec::new(),
					 parent: None,
					 cells: ChartTimeRow::new() }
	}

	fn create_attribute(&mut self, key: &str, val: &str) {
		self.attributes.insert(key.to_string(), val.to_string());
	}

	fn new_child(&mut self, name: &str, indent: u32, line_num: u32) {
		self.children.push(Rc::new(RefCell::new(ConfigNode::new(name, 
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

	pub fn find_child_with_name(&self, name: &str) -> 
										Option<Weak<RefCell<ConfigNode>>> {

		if self.name == name {
			return None;
		} else {
			for child_rc in &self.children {
				if child_rc.borrow().name == name {
					return Some(Rc::downgrade(child_rc));
				} 
			}
		}

		None

	}

	pub fn get_inherited_attribute(&self, key: &str) -> Option<String> {

		if self.attributes.contains_key(key) {
			return Some(self.attributes[key].clone());
		}
		else {
			match self.parent {
				None => None,
				Some(ref p) => {
					match p.upgrade() {
						None => None,
						Some(node) => 
									node.borrow().get_inherited_attribute(key)
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
		let mut h = HashMap::new();

		for (key, val) in self.attributes.iter() {
			h.insert(key.clone(), val.clone());
		}

		h
	}

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

	pub fn get_people(&self, weeks: u32) -> HashMap<String, ChartTimeRow> {
		
		match self.find_child_with_name("[people]") {
			None => HashMap::new(),
			Some(node) => {
				match node.upgrade() {
					None => HashMap::new(),
					Some(n) => {
						let mut h = HashMap::new();
						for (key, val) in n.borrow().attributes.iter() {
							match ChartTimeRow::new_populate_range(val, 
																   weeks) {
								Ok(ct) => {
									h.insert(key.clone(), ct);
								},
								Err(e) => {
									println!("Problem setting up resource for {}: {}", 
										     key, 
										     e.to_string());
								}
							}
						}
						h
					}
				}
			}
		}
	}

	/// Get a configuration value
	pub fn get_config_val<T>(&self, key: &str, default: Option<T>) -> Result<T, String> 
		where T: FromStr, 
		      <T as FromStr>::Err: Display {

		// Get global config
		let global_hash = self.get_global_config();

		// Read in resource information ([people])
		match global_hash.get(key) {
			Some(k) => {
				match k.parse::<T>() {
					Ok(v) => Ok(v),
					Err(e) => Err(format!("Problem parsing config {}: {}", key, e.to_string()))
				}
			},
			None => match default {
				Some(x) => Ok(x),
				None => Err(format!("No config in [chart] for {}", key))
			}
		}
	}

	pub fn transfer_committed_resource(&mut self, people_hash: &mut HashMap<String, ChartTimeRow>) -> Result<(), String> {

		let valid_who: Vec<String> = people_hash.keys().map(|x| x.clone()).collect();
		for (start, duration) in self.get_commitments() {
			let who = try!(self.get_who(&valid_who).ok_or(format!("Node at line {} has commitments but no owner", self.line_num)));
			match people_hash.get_mut(&who).unwrap().fill_transfer_to(&mut self.cells, duration.quarters() as u32, start.get_quarter() .. (start.get_quarter() + (duration.quarters() as u32) - 1)) {
				(_, _, 0) => {
					continue;
				},
				_ => {
					return Err(format!("Unable to transfer resource for node at line {}", self.line_num))
				}
			}
		}

		// Now do any child nodes
		for child_rc in &self.children {
			try!(child_rc.borrow_mut().transfer_committed_resource(people_hash));
		}

		Ok(())
	}

	/// Set up gantt information in the chart
	///
	/// This is only called on the root node.
	pub fn fill_in_gantt(&mut self) -> Result<(), String> {

		// Read in resource information ([people])
		let weeks: u32 = try!(self.get_config_val("weeks", None));
		let mut people_hash = self.get_people(weeks);

		// Move committed resource into the cells
		try!(self.transfer_committed_resource(&mut people_hash));

		// Handle all non-managed rows

		// Handle Management

		// Handle all managed rows

		Ok(())
	}

	// Functions to derive resourcing information
	// Derive Remaining and slip/gain
	// Draw the Gantt!

	/// Get the non-managed status for the tasktask.
	///
	/// Non-managed status is inheritable, and defaults
	/// to false.     
	pub fn get_non_managed(&self) -> bool {
		match self.get_inherited_attribute("non-managed") {
			Some(val) => {
				if val == "true" {
					true
				} else {
					false
				}
			},
			None => false
		}
	}

	/// Get the latest end time for the task.
	///
	/// Latest end time is inheritable, and is
	/// not defaulted.
	pub fn get_latest_end(&self) -> Option<ChartTime> {
		match self.get_inherited_attribute("latest-end") {
			Some(ref time) => {
				match ChartTime::new(time) {
					Ok(ct) => Some(ct),
					Err(e) => {
						println!("Invalid end time in node at line {}: {}", self.line_num, e.to_string());
						None
					}
				}
			},
			None => None
		}
	}

	/// Get the earliest start time for the task.
	///
	/// Earliest start time is inheritable, and is
	/// not defaulted.
	pub fn get_earliest_start(&self) -> Option<ChartTime> {
		match self.get_inherited_attribute("earliest-start") {
			Some(ref time) => {
				match ChartTime::new(time) {
					Ok(ct) => Some(ct),
					Err(e) => {
						println!("Invalid start time in node at line {}: {}", self.line_num, e.to_string());
						None
					}
				}
			},
			None => None
		}
	}

	/// Get the resourcing commitments on this node
	///
	/// Commitments are not inheritable
	pub fn get_commitments(&self) -> Vec<(ChartTime, Duration)> {

		// Locate attributes that represent a commitment, and
		// build a map from ChartTime to duration.
		let mut map = BTreeMap::new();
		for (key, value) in &self.attributes {
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
	pub fn get_resourcing_strategy(&self) -> Option<ResourcingStrategy> {
		match self.get_inherited_attribute("resource") {
			Some(resource) => {
				if resource == "management" {
					Some(ResourcingStrategy::Management)
				} else if resource == "smearprorata" {
					Some(ResourcingStrategy::SmearProRata)
				} else if resource == "smearremaining" {
					Some(ResourcingStrategy::SmearRemaining)
				} else if resource == "frontload" {
					Some(ResourcingStrategy::FrontLoad)
				} else if resource == "backload" {
					Some(ResourcingStrategy::BackLoad)
				} else if resource == "prodsfr" {
					Some(ResourcingStrategy::ProdSFR)
				} else {
					println!("Invalid scheduling strategy in node at line {}", self.line_num);
					None
				}
			},

			None => {
				None
			}
		}
	}

	/// Get the approach used to schedule the child nodes
	///
	/// The scheduling approach is not inherited.  By default.
	/// a nodes children are scheduled in parallel
	pub fn get_scheduling_strategy(&self) -> SchedulingStrategy {
		
		let key = "schedule";
		if self.attributes.contains_key(key) {

			if self.attributes[key] == "parallel" {
				SchedulingStrategy::Parallel
			} else if self.attributes[key] == "serial" {
				SchedulingStrategy::Serial
			} else {
				println!("Invalid scheduling strategy in node at line {}", self.line_num);
				SchedulingStrategy::Parallel
			}
		}
		else {
			SchedulingStrategy::Parallel
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
	///	The planned time, as well as being a number, can also be suffixed with pcy or
	/// pcm.  This function converts suffixed values into actual durations.
	pub fn get_plan(&self, when: &ChartTime, time_in_chart: &Duration) -> Option<Duration> {

		let key = "plan";
		let plan_str: String;
		if self.attributes.contains_key(key) {
			plan_str = self.attributes[key].clone();
		}
		else if self.is_leaf() {

			match self.get_inherited_attribute("default-plan") {
				Some(val) => {
					plan_str = val.clone();
				},

				None => { 
					return None; 
				}
			};
		}
		else {
			return None;
		}

		// If plan_str contains multiple values, use the "when" time to select
		// the value that applies.
		let v: Vec<&str> = plan_str.split(", ").collect();
		let mut use_val: &str = "";
		let mut found = false;
		for val in v {
			let v2: Vec<&str> = val.split(":").collect();
			if v2.len() > 2 {
				println!("Invalid plan part in node at line {}: {} has more than 2 parts", self.line_num, val);
				return None;
			}
			if v2.len() == 1 {
				found = true;
				use_val = val;
				continue;
			}
			match ChartTime::new(v2[0]) {
				Err(e) => {
					println!("Invalid plan part in node at line {}: {}", self.line_num, e.to_string());
					return None;
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
			return None;
		}

		// So, we have a value in use_val.  Try to convert it to a duration.
		match Duration::new_from_string(&use_val, time_in_chart) {
			Err(e) => {
				println!("Invalid plan in node at line {}: {}", self.line_num, e);
				None
			},
			Ok(dur) => {
				Some(dur)
			}
		}
	}

	/// Get the owner of this task
	///
	/// The owner can be inherited.  If this fails, the name of the
	/// node is returned as an owner.
	pub fn get_who(&self, valid: &Vec<String>) -> Option<String> {

		match self.get_inherited_attribute("who") {
			Some(who) => {
				if valid.contains(&who) {
					return Some(who);
				}
				else {
					println!("Invalid who in (or inherited by) node at line {}", self.line_num);
					return None;
				}
			},

			None => {
				if valid.contains(&self.name) {
					return Some(self.name.clone());
				}
				else {
					return None
				}
			}
		};
	}

	/// Get the budget, as a Duration.
	///
	/// The buget is not inherited - it is set against a single node, then
	/// compared with the plan/gain/commitments of all children.
	pub fn get_budget(&self) -> Option<Duration> {

		let key = "budget";
		if !self.attributes.contains_key(key) {
			return None;
		}

		match self.attributes[key].parse::<f32>() {
			Err(e) => {
				println!("Invalid budget in node at line {}: {}", self.line_num, e.to_string());
				return None;
			}
			Ok(dur) => {
				return Some(Duration::new_days(dur));
			}
		}
	}

	/// Return a weak reference to a child Node at a given line in the config file.
	pub fn get_node_at_line(&self, 
						    line_num: u32) -> Option<Weak<RefCell<ConfigNode>>> {

		if self.line_num == line_num {
			return None;
		}
		else {
			for child_rc in &self.children {
				if child_rc.borrow().line_num == line_num {
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

	pub fn consume_config(&mut self, 
						  ref_self: Option<&Rc<RefCell<ConfigNode>>>, 
						  file: &mut ConfigLines) -> Result<(), String> {

		loop {
			match file.peek_line() {
				Some(Line::Node(LineNode {line_num, indent, name } )) => {
					if self.indent >= indent {
						break;
					}

					file.get_line().unwrap();
					self.new_child(&name, indent, line_num);

					// Use child of our parent?  Maybe via find_node_at_line?  Or have the Rc<<>> in scope to use here?
					let new_child = &self.children[self.children.len() - 1];
					new_child.borrow_mut().parent = match ref_self {
						None => None,
						Some(rc) => Some(Rc::downgrade(rc))
					};
					new_child.borrow_mut().consume_config(Some(&new_child), file).unwrap();
				}
				Some(Line::Attribute(LineAttribute {key, value } )) => {
					self.create_attribute(&key, &value);
					file.get_line().unwrap();
				}
				None => { break; }
			};
		}

		Ok(())
	}
}
	