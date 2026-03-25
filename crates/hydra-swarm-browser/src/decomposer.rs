//! Task decomposer — breaks a high-level goal into N parallel sub-tasks.
//! Uses LLM if available, heuristic fallback if not.

use crate::constants::MAX_SUBTASKS_PER_GOAL;
use crate::types::*;

/// Decompose a goal into parallel sub-tasks.
pub fn decompose(goal: &SwarmGoal) -> Vec<SwarmTask> {
    let mut tasks = Vec::new();
    let desc = &goal.description;

    // Extract explicit URLs first
    tasks.extend(extract_url_tasks(goal));

    // Generate faceted research queries for remaining slots
    let remaining = goal.max_workers.min(MAX_SUBTASKS_PER_GOAL).saturating_sub(tasks.len());
    if remaining > 0 {
        let topic = extract_topic(desc);
        tasks.extend(generate_faceted_queries(&topic, goal.id, remaining));
    }

    tasks.truncate(goal.max_workers.min(MAX_SUBTASKS_PER_GOAL));
    eprintln!("hydra-swarm: decomposed '{}' into {} tasks", desc, tasks.len());
    tasks
}

/// Extract YouTube and document URLs from the goal.
fn extract_url_tasks(goal: &SwarmGoal) -> Vec<SwarmTask> {
    let mut tasks = Vec::new();
    for word in goal.description.split_whitespace() {
        let clean = word.trim_end_matches(|c: char| ".,;:!?)\"'".contains(c));
        if clean.contains("youtube.com/watch") || clean.contains("youtu.be/") {
            tasks.push(SwarmTask::new(
                goal.id, clean,
                SwarmTaskType::YouTubeTranscript { video_url: clean.to_string() },
            ));
        } else if clean.starts_with("http://") || clean.starts_with("https://") {
            tasks.push(SwarmTask::new(
                goal.id, clean,
                SwarmTaskType::DeepRead { url: clean.to_string() },
            ));
        }
    }
    // EC-20.4: Deduplicate tasks by query to prevent multiple agents hitting the same page
    dedup_by_query(&mut tasks);
    tasks
}

/// Remove duplicate tasks that would visit the same URL or run the same query.
fn dedup_by_query(tasks: &mut Vec<SwarmTask>) {
    let mut seen = std::collections::HashSet::new();
    tasks.retain(|t| seen.insert(t.query.clone()));
}

/// Extract the core topic from a goal description (strip action words).
fn extract_topic(desc: &str) -> String {
    let action_words = ["learn", "research", "study", "understand", "explore", "find",
        "search", "watch", "read", "about", "how", "what", "why", "does", "the",
        "a", "to", "for", "from", "on", "in", "of", "and", "or", "is"];
    desc.split_whitespace()
        .filter(|w| !action_words.contains(&w.to_lowercase().as_str()))
        .filter(|w| !w.starts_with("http"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Generate faceted research queries for a topic.
fn generate_faceted_queries(topic: &str, goal_id: uuid::Uuid, count: usize) -> Vec<SwarmTask> {
    let facets = [
        ("{topic} overview explanation", SwarmTaskType::WebSearch),
        ("{topic} tutorial examples", SwarmTaskType::WebSearch),
        ("{topic} latest developments 2026", SwarmTaskType::WebSearch),
        ("{topic} comparison alternatives", SwarmTaskType::WebSearch),
        ("{topic} code implementation github", SwarmTaskType::WebSearch),
    ];

    facets.iter()
        .take(count)
        .map(|(template, task_type)| {
            let query = template.replace("{topic}", topic);
            SwarmTask::new(goal_id, &query, task_type.clone())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decompose_plain_topic() {
        let goal = SwarmGoal::new("learn about quantum computing", 5);
        let tasks = decompose(&goal);
        assert!(tasks.len() >= 3);
        assert!(tasks.iter().all(|t| matches!(t.task_type, SwarmTaskType::WebSearch)));
    }

    #[test]
    fn decompose_with_youtube_url() {
        let goal = SwarmGoal::new("watch https://youtube.com/watch?v=abc123 and learn", 5);
        let tasks = decompose(&goal);
        assert!(tasks.iter().any(|t| matches!(t.task_type, SwarmTaskType::YouTubeTranscript { .. })));
    }

    #[test]
    fn decompose_with_doc_url() {
        let goal = SwarmGoal::new("read https://docs.rust-lang.org/book/", 3);
        let tasks = decompose(&goal);
        assert!(tasks.iter().any(|t| matches!(t.task_type, SwarmTaskType::DeepRead { .. })));
    }

    #[test]
    fn extract_topic_strips_action_words() {
        assert_eq!(extract_topic("learn about quantum computing"), "quantum computing");
        assert_eq!(extract_topic("research rust ownership model"), "rust ownership model");
    }
}
