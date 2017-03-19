# Planner

A project planning tool.

Currently, very much in development!

## Ideas for next development

* Split nodes.rs into:
  * structs, new, getters, setters
  * creation
  * general resource allocation
  * management resource allocation
  * display logic
* Add a class for ChartPeriod - all we have at the moment are
  a time and duration.
* For each user, keep a period_available
* Rethink periods in smearprorata
  * Smear over intersection of 
    * period remaining
    * period this user is available
* Move logic for creating cell rows out of the template
* Add column borders and labels for key dates
* Add budget support
* Add new views