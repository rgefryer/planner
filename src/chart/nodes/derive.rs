use super::super::duration::*;
use super::super::time::*;
use super::super::timerow::*;
use super::super::SchedulingStrategy;
use super::super::ResourcingStrategy;
use super::*;

impl ConfigNode {
    pub fn get_inherited_attribute<T>(&self, key: &str) -> Result<Option<T>, String>
        where T: FromStr,
              <T as FromStr>::Err: Display
    {

        if self.data
               .borrow()
               .attributes
               .contains_key(key) {
            return self.data.borrow().attributes[key]
                       .parse::<T>()
                       .map_err(|e| {
                                    format!("Problem parsing config {} on node at line {}: {}",
                                            key,
                                            self.data.borrow().line_num,
                                            e.to_string())
                                })
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
                        Some(node) => node.borrow().get_inherited_attribute(key),
                    }
                }
            }
        }
    }


    /// Get the non-managed status for the tasktask.
    ///
    /// Non-managed status is inheritable, and defaults
    /// to false.
    pub fn get_non_managed(&self) -> Result<bool, String> {
        match self.get_inherited_attribute("non-managed") {
            Ok(Some(val)) => Ok(val),
            Ok(None) => Ok(false),
            Err(e) => Err(self.augment_error(e)),
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
                    Err(e) => Err(self.augment_error(e)),
                }
            }
            Ok(None) => Ok(None),
            Err(e) => Err(self.augment_error(e)),
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
                    Err(e) => Err(self.augment_error(e)),
                }
            }
            Ok(None) => Ok(None),
            Err(e) => Err(self.augment_error(e)),
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
                match ChartTime::new(&key[1..]) {
                    Ok(ct) => {
                        match value.parse::<f32>() {
                            Ok(d) => {
                                map.insert(ct, Duration::new_days(d));
                            }
                            Err(_) => {
                                continue;
                            }
                        }
                    }
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
            }

            Ok(None) => Ok(None),
            Err(e) => Err(self.augment_error(e)),
        }
    }

    /// Get the approach used to schedule the child nodes
    ///
    /// The scheduling approach is not inherited.  By default.
    /// a nodes children are scheduled in parallel
    pub fn get_scheduling_strategy(&self) -> Result<SchedulingStrategy, String> {

        let key = "schedule";
        if self.data
               .borrow()
               .attributes
               .contains_key(key) {

            if self.data.borrow().attributes[key] == "parallel" {
                Ok(SchedulingStrategy::Parallel)
            } else if self.data.borrow().attributes[key] == "serial" {
                Ok(SchedulingStrategy::Serial)
            } else {
                Err(format!("Invalid scheduling strategy: {}",
                            self.data.borrow().attributes[key]))
            }
        } else {
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
                          than 2 parts",
                                                      val)));
            }
            if v2.len() == 1 {
                found = true;
                use_val = val;
                continue;
            }
            match ChartTime::new(v2[0]) {
                Err(e) => {
                    return Err(self.augment_error(e));
                }
                Ok(ref ct) => {
                    if ct > when {
                        break;
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
            Err(e) => Err(self.augment_error(e)),
            Ok(dur) => Ok(Some(dur)),
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
    /// For this reason, the function requires a date, `when`, to
    /// specify the point for which the planned date is required.
    ///
    /// The planned time, as well as being a number, can also be suffixed with
    /// pcy or pcm.  This function converts suffixed values into actual
    /// durations.
    pub fn get_plan(&self,
                    when: &ChartTime,
                    time_in_chart: &Duration)
                    -> Result<Option<Duration>, String> {

        // Try to satisfy the request with local data
        let key = "plan";
        if self.data
               .borrow()
               .attributes
               .contains_key(key) {
            match self.plan_string_to_dur(&self.data.borrow().attributes[key],
                                          when,
                                          time_in_chart) {
                Ok(Some(d)) => {
                    return Ok(Some(d));
                }
                Ok(None) => {}
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
                return self.plan_string_to_dur(&val, when, time_in_chart);
            }

            Ok(None) => {
                return Ok(None);
            }

            Err(e) => {
                return Err(e);
            }
        };
    }

    /// Get a map from people to timerows.
    ///
    /// It is an error if there are none defined, or if any are badly defined.
    pub fn get_people(&self, weeks: u32) -> Result<HashMap<String, ChartTimeRow>, String> {

        let weak_node = try!(self.find_child_with_name("[people]")
                                 .ok_or("[people] node must exist"));
        let node_rc = weak_node.upgrade().unwrap();
        let node = node_rc.borrow();

        let mut people_hash = HashMap::new();
        for (key, val) in node.data
                .borrow()
                .attributes
                .iter() {
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
    pub fn get_config_val<T>(&self, key: &str, default: Option<T>) -> Result<T, String>
        where T: FromStr,
              <T as FromStr>::Err: Display
    {

        // Get global config
        let global_hash = self.get_global_config();

        // Read in resource information ([people])
        match global_hash.get(key) {
            Some(k) => {
                match k.parse::<T>() {
                    Ok(v) => Ok(v),
                    Err(e) => Err(format!("Problem parsing config {}: {}", key, e.to_string())),
                }
            }
            None => {
                match default {
                    Some(x) => Ok(x),
                    None => Err(format!("No config in [chart] for {}", key)),
                }
            }
        }
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
            }

            Ok(None) => {
                if valid.contains(&self.data.borrow().name) {
                    return Ok(Some(self.data
                                       .borrow()
                                       .name
                                       .clone()));
                } else {
                    return Ok(None);
                }
            }

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
        if !self.data
                .borrow()
                .attributes
                .contains_key(key) {
            return None;
        }

        match self.data.borrow().attributes[key].parse::<f32>() {
            Err(e) => {
                println!("Invalid budget in node at line {}: {}",
                         self.data.borrow().line_num,
                         e.to_string());
                return None;
            }
            Ok(dur) => {
                return Some(Duration::new_days(dur));
            }
        }
    }
}
