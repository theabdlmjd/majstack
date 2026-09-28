use majstack_core::{MajstackError, Result, TaskStatus};
use majstack_state::TaskRecord;
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct Dag {
    tasks: BTreeMap<String, TaskRecord>,
    dependencies: HashMap<String, Vec<String>>,
}

impl Dag {
    pub fn build(tasks: &[TaskRecord], edges: &[(String, String)]) -> Result<Self> {
        let mut task_map = BTreeMap::new();
        for task in tasks {
            task_map.insert(task.id.clone(), task.clone());
        }
        let mut dependencies: HashMap<String, Vec<String>> = HashMap::new();
        for task in tasks {
            dependencies.entry(task.id.clone()).or_default();
        }
        for (task_id, depends_on) in edges {
            if task_id == depends_on {
                return Err(MajstackError::Invalid(format!(
                    "task {task_id} depends on itself"
                )));
            }
            if !task_map.contains_key(task_id) {
                continue;
            }
            if !task_map.contains_key(depends_on) {
                return Err(MajstackError::Invalid(format!(
                    "task {task_id} depends on unknown task {depends_on}"
                )));
            }
            dependencies
                .entry(task_id.clone())
                .or_default()
                .push(depends_on.clone());
        }
        let dag = Dag {
            tasks: task_map,
            dependencies,
        };
        if let Some(cycle) = dag.find_cycle() {
            return Err(MajstackError::Invalid(format!(
                "dependency cycle detected: {}",
                cycle.join(" -> ")
            )));
        }
        Ok(dag)
    }

    pub fn get(&self, id: &str) -> Option<&TaskRecord> {
        self.tasks.get(id)
    }

    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    pub fn deps_of(&self, id: &str) -> &[String] {
        self.dependencies.get(id).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn dependents_of(&self, id: &str) -> Vec<String> {
        self.dependencies
            .iter()
            .filter(|(_, deps)| deps.iter().any(|d| d == id))
            .map(|(task, _)| task.clone())
            .collect()
    }

    pub fn find_cycle(&self) -> Option<Vec<String>> {
        let mut visited: HashSet<String> = HashSet::new();
        let mut stack: Vec<String> = Vec::new();
        let mut on_stack: HashSet<String> = HashSet::new();
        for id in self.tasks.keys() {
            if let Some(cycle) = self.dfs(id, &mut visited, &mut stack, &mut on_stack) {
                return Some(cycle);
            }
        }
        None
    }

    fn dfs(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        stack: &mut Vec<String>,
        on_stack: &mut HashSet<String>,
    ) -> Option<Vec<String>> {
        if on_stack.contains(node) {
            let start = stack.iter().position(|n| n == node).unwrap_or(0);
            let mut cycle = stack[start..].to_vec();
            cycle.push(node.to_string());
            return Some(cycle);
        }
        if visited.contains(node) {
            return None;
        }
        visited.insert(node.to_string());
        stack.push(node.to_string());
        on_stack.insert(node.to_string());
        for dep in self.deps_of(node) {
            if let Some(cycle) = self.dfs(dep, visited, stack, on_stack) {
                return Some(cycle);
            }
        }
        stack.pop();
        on_stack.remove(node);
        None
    }

    pub fn is_ready(&self, id: &str) -> bool {
        let task = match self.tasks.get(id) {
            Some(task) => task,
            None => return false,
        };
        if matches!(
            task.status,
            TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled
        ) {
            return false;
        }
        if task.max_attempts > 0 && task.attempts >= task.max_attempts {
            return false;
        }
        self.deps_of(id).iter().all(|dep| {
            self.tasks
                .get(dep)
                .map(|task| task.status == TaskStatus::Completed)
                .unwrap_or(false)
        })
    }

    pub fn ready(&self) -> Vec<&TaskRecord> {
        let mut ready: Vec<&TaskRecord> = self
            .tasks
            .values()
            .filter(|task| {
                matches!(
                    task.status,
                    TaskStatus::Pending | TaskStatus::Ready | TaskStatus::Blocked
                )
            })
            .filter(|task| self.is_ready(&task.id))
            .collect();
        ready.sort_by_key(|task| (task.priority, task.created_at));
        ready
    }

    pub fn next(&self) -> Option<&TaskRecord> {
        self.ready().into_iter().next()
    }

    pub fn ready_batch(&self, limit: usize) -> Vec<&TaskRecord> {
        self.ready().into_iter().take(limit).collect()
    }

    pub fn blocked(&self) -> Vec<&TaskRecord> {
        self.tasks
            .values()
            .filter(|task| {
                matches!(
                    task.status,
                    TaskStatus::Pending | TaskStatus::Ready | TaskStatus::Blocked
                ) && !self.is_ready(&task.id)
            })
            .collect()
    }

    pub fn children_of(&self, parent: &str) -> Vec<&TaskRecord> {
        self.tasks
            .values()
            .filter(|task| task.parent_id.as_deref() == Some(parent))
            .collect()
    }

    pub fn rollup(&self, parent: &str) -> Option<TaskStatus> {
        let children = self.children_of(parent);
        if children.is_empty() {
            return None;
        }
        let all_done = children.iter().all(|c| c.status == TaskStatus::Completed);
        if all_done {
            return Some(TaskStatus::Completed);
        }
        let all_terminal = children.iter().all(|c| c.status.is_terminal());
        if all_terminal && children.iter().any(|c| c.status == TaskStatus::Failed) {
            return Some(TaskStatus::Failed);
        }
        None
    }

    pub fn unresolved(&self) -> usize {
        self.tasks
            .values()
            .filter(|task| !task.status.is_terminal())
            .count()
    }

    pub fn all_resolved(&self) -> bool {
        self.tasks.values().all(|task| task.status.is_terminal())
    }

    pub fn roots(&self) -> Vec<&TaskRecord> {
        self.tasks
            .values()
            .filter(|task| self.deps_of(&task.id).is_empty())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use majstack_state::NewTask;

    fn task(id: &str, status: TaskStatus) -> TaskRecord {
        let mut record = TaskRecord {
            id: id.to_string(),
            run_id: "r".into(),
            parent_id: None,
            title: id.to_string(),
            description: None,
            objective: None,
            acceptance_criteria: None,
            status,
            priority: 0,
            risk: None,
            complexity: None,
            capability: None,
            provider: None,
            model: None,
            workspace: None,
            attempts: 0,
            max_attempts: 0,
            verify_required: true,
            evidence_required: false,
            ui: false,
            claimed_by: None,
            notes: None,
            created_at: 0,
            updated_at: 0,
            started_at: None,
            finished_at: None,
            meta: None,
        };
        record.status = status;
        record
    }

    #[test]
    fn rejects_cycles() {
        let tasks = vec![
            task("a", TaskStatus::Pending),
            task("b", TaskStatus::Pending),
        ];
        let edges = vec![
            ("a".to_string(), "b".to_string()),
            ("b".to_string(), "a".to_string()),
        ];
        assert!(Dag::build(&tasks, &edges).is_err());
    }

    #[test]
    fn only_unblocked_tasks_are_ready() {
        let tasks = vec![
            task("a", TaskStatus::Completed),
            task("b", TaskStatus::Pending),
            task("c", TaskStatus::Pending),
        ];
        let edges = vec![
            ("b".to_string(), "a".to_string()),
            ("c".to_string(), "b".to_string()),
        ];
        let dag = Dag::build(&tasks, &edges).unwrap();
        let ready: Vec<&str> = dag.ready().iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ready, vec!["b"]);
    }

    #[test]
    fn respects_attempt_limits() {
        let mut exhausted = task("a", TaskStatus::Failed);
        exhausted.attempts = 3;
        exhausted.max_attempts = 3;
        let dag = Dag::build(&[exhausted], &[]).unwrap();
        assert!(dag.ready().is_empty());
    }

    #[test]
    fn rolls_up_parent_status() {
        let mut parent = task("p", TaskStatus::Running);
        parent.parent_id = None;
        let mut child = task("c", TaskStatus::Pending);
        child.parent_id = Some("p".to_string());
        let mut dag = Dag::build(&[parent.clone(), child.clone()], &[]).unwrap();
        assert_eq!(dag.rollup("p"), None);
        dag.tasks.get_mut("c").unwrap().status = TaskStatus::Completed;
        assert_eq!(dag.rollup("p"), Some(TaskStatus::Completed));
        dag.tasks.get_mut("c").unwrap().status = TaskStatus::Failed;
        assert_eq!(dag.rollup("p"), Some(TaskStatus::Failed));
    }

    #[test]
    fn detects_next_by_priority() {
        let mut low = task("low", TaskStatus::Pending);
        low.priority = 5;
        let mut high = task("high", TaskStatus::Pending);
        high.priority = 1;
        let dag = Dag::build(&[low, high], &[]).unwrap();
        assert_eq!(dag.next().unwrap().id, "high");
    }

    #[test]
    fn builds_unknown_dependency_errors() {
        let tasks = vec![task("a", TaskStatus::Pending)];
        let edges = vec![("a".to_string(), "ghost".to_string())];
        assert!(Dag::build(&tasks, &edges).is_err());
    }

    #[test]
    fn new_task_defaults_are_pending() {
        let task = NewTask {
            run_id: "r".into(),
            title: "t".into(),
            ..NewTask::default()
        };
        assert_eq!(task.title, "t");
    }
}
