use overclock_core::event::{EventBus, OrchestratorEvent};
use crossterm::{cursor, execute, terminal, event::{self, Event, KeyCode, EnableMouseCapture, DisableMouseCapture, MouseEventKind}};
use std::io::stdout;
use std::collections::{VecDeque, HashMap, HashSet};
use ratatui::{backend::CrosstermBackend, layout::{Constraint, Direction, Layout}, widgets::{Block, Borders, List, ListItem, Paragraph}, style::{Style, Color, Modifier}, Terminal};
use uuid;

pub async fn run_monitor() -> anyhow::Result<()> {
    // Initialize terminal
    let mut stdout = stdout();
    execute!(
        stdout,
        terminal::EnterAlternateScreen,
        cursor::Hide,
        terminal::Clear(terminal::ClearType::All),
        EnableMouseCapture
    )?;

    // Create backend and terminal
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create event bus (in a real scenario, this would be passed from the main app)
    let event_bus = EventBus::default();
    let mut receiver = event_bus.subscribe();

    // Store recent events (limited to 100 events)
    let mut recent_events: VecDeque<(String, Color, OrchestratorEvent)> = VecDeque::with_capacity(100);
    recent_events.push_back(("Waiting for events...".to_string(), Color::White, OrchestratorEvent::TaskCreated { task_id: uuid::Uuid::new_v4(), title: "Initializing".to_string(), role: "system".to_string() }));

    // Task statistics
    let mut task_stats: HashMap<String, u32> = HashMap::new();

    // Event filters
    let mut event_filters: HashSet<String> = HashSet::from([
        "Task Created".to_string(),
        "Task Assigned".to_string(),
        "Agent Started".to_string(),
        "Agent Output".to_string(),
        "Task Completed".to_string(),
        "Task Failed".to_string(),
        "Workflow Started".to_string(),
        "Workflow Completed".to_string(),
    ]);

    // Selected event index for detailed view
    let mut selected_event_index: Option<usize> = None;

    // Main loop
    loop {
        // Draw UI
        terminal.draw(|f| {
            let size = f.size();
            
            // Layout
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .margin(1)
                .constraints([
                    Constraint::Ratio(2, 3),
                    Constraint::Ratio(1, 3),
                ])
                .split(size);

            // Left column (events)
            let left_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(15),
                    Constraint::Length(3),
                    Constraint::Min(10),
                ])
                .split(main_chunks[0]);

            // Right column (stats and filters)
            let right_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(10),
                    Constraint::Length(3),
                    Constraint::Min(15),
                ])
                .split(main_chunks[1]);

            // Header
            let header = Paragraph::new("Overclock-AI Monitor")
                .block(Block::default().borders(Borders::ALL).title("Status"))
                .style(Style::default().fg(Color::Cyan));
            f.render_widget(header, left_chunks[0]);

            // Events list
            let events: Vec<ListItem> = recent_events
                .iter()
                .enumerate()
                .map(|(i, (event, color, _))| {
                    let style = if selected_event_index == Some(i) {
                        Style::default().fg(*color).add_modifier(Modifier::REVERSED)
                    } else {
                        Style::default().fg(*color)
                    };
                    ListItem::new(event.clone()).style(style)
                })
                .collect();
            let events_list = List::new(events)
                .block(Block::default().borders(Borders::ALL).title(format!("Recent Events ({} events)", recent_events.len())));
            f.render_widget(events_list, left_chunks[1]);

            // Event details
            if let Some(index) = selected_event_index {
                if let Some((_, _, event)) = recent_events.get(index) {
                    let details = format_event_details(event);
                    let details_paragraph = Paragraph::new(details)
                        .block(Block::default().borders(Borders::ALL).title("Event Details"))
                        .style(Style::default().fg(Color::White));
                    f.render_widget(details_paragraph, left_chunks[3]);
                }
            } else {
                let placeholder = Paragraph::new("Select an event to see details")
                    .block(Block::default().borders(Borders::ALL).title("Event Details"))
                    .style(Style::default().fg(Color::Gray));
                f.render_widget(placeholder, left_chunks[3]);
            }

            // Footer
            let footer = Paragraph::new("Press 'q' to quit | 'c' to clear events | '↑'/'↓' to select event")
                .block(Block::default().borders(Borders::ALL).title("Controls"));
            f.render_widget(footer, left_chunks[2]);

            // Stats header
            let stats_header = Paragraph::new("Task Statistics")
                .block(Block::default().borders(Borders::ALL).title("Stats"))
                .style(Style::default().fg(Color::Green));
            f.render_widget(stats_header, right_chunks[0]);

            // Stats list
            let mut stats_items: Vec<ListItem> = vec![];
            for (task_type, count) in &task_stats {
                stats_items.push(ListItem::new(format!("{}: {}", task_type, count)));
            }
            if stats_items.is_empty() {
                stats_items.push(ListItem::new("No tasks yet"));
            }
            let stats_list = List::new(stats_items)
                .block(Block::default().borders(Borders::ALL).title("Task Counts"));
            f.render_widget(stats_list, right_chunks[1]);

            // Filters header
            let filters_header = Paragraph::new("Event Filters")
                .block(Block::default().borders(Borders::ALL).title("Filters"))
                .style(Style::default().fg(Color::Yellow));
            f.render_widget(filters_header, right_chunks[2]);

            // Filters list
            let filter_items = vec![
                "Task Created", "Task Assigned", "Agent Started", "Agent Output",
                "Task Completed", "Task Failed", "Workflow Started", "Workflow Completed"
            ];
            let mut filter_list_items: Vec<ListItem> = vec![];
            for filter in &filter_items {
                let checked = event_filters.contains(&filter.to_string());
                let status = if checked { "[x]" } else { "[ ]" };
                let filter_text = format!("{} {}", status, filter);
                filter_list_items.push(ListItem::new(filter_text));
            }
            let filter_list = List::new(filter_list_items)
                .block(Block::default().borders(Borders::ALL).title("Toggle with Space"));
            f.render_widget(filter_list, right_chunks[3]);
        })?;

        // Check for events
        tokio::select! {
            Ok(event) = receiver.recv() => {
                // Check if event type is filtered
                let event_type = get_event_type(&event);
                if event_filters.contains(&event_type) {
                    // Handle event
                    let (event_str, color) = format_event_with_color(&event);
                    recent_events.push_back((event_str, color, event.clone()));
                    if recent_events.len() > 100 {
                        recent_events.pop_front();
                    }

                    // Update task statistics
                    update_task_stats(&event, &mut task_stats);
                }
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                // Check for user input
                if event::poll(std::time::Duration::from_millis(10))? {
                    match event::read()? {
                        Event::Key(key) => {
                            match key.code {
                                KeyCode::Char('q') => break,
                                KeyCode::Char('c') => {
                                    recent_events.clear();
                                    recent_events.push_back(("Events cleared".to_string(), Color::White, OrchestratorEvent::TaskCreated { task_id: uuid::Uuid::new_v4(), title: "Initializing".to_string(), role: "system".to_string() }));
                                    task_stats.clear();
                                    selected_event_index = None;
                                }
                                KeyCode::Char(' ') => {
                                    // Toggle filter for selected event type
                                    if let Some(index) = selected_event_index {
                                        if let Some((_, _, event)) = recent_events.get(index) {
                                            let event_type = get_event_type(event);
                                            if event_filters.contains(&event_type) {
                                                event_filters.remove(&event_type);
                                            } else {
                                                event_filters.insert(event_type);
                                            }
                                        }
                                    }
                                }
                                KeyCode::Up => {
                                    // Select previous event
                                    if let Some(index) = selected_event_index {
                                        if index > 0 {
                                            selected_event_index = Some(index - 1);
                                        }
                                    } else if !recent_events.is_empty() {
                                        selected_event_index = Some(recent_events.len() - 1);
                                    }
                                }
                                KeyCode::Down => {
                                    // Select next event
                                    if let Some(index) = selected_event_index {
                                        if index < recent_events.len() - 1 {
                                            selected_event_index = Some(index + 1);
                                        }
                                    } else if !recent_events.is_empty() {
                                        selected_event_index = Some(0);
                                    }
                                }
                                _ => {}
                            }
                        }
                        Event::Mouse(mouse_event) => {
                            // Handle mouse events
                            match mouse_event.kind {
                                MouseEventKind::Down(_) => {
                                    // Handle mouse click to select event or toggle filter
                                    // This is a simplified implementation
                                    // In a real app, we would calculate which UI element was clicked
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // Cleanup
    execute!(
        terminal.backend_mut(),
        cursor::Show,
        terminal::LeaveAlternateScreen,
        DisableMouseCapture
    )?;

    Ok(())
}

/// Format an orchestrator event into a human-readable string with color
fn format_event_with_color(event: &OrchestratorEvent) -> (String, Color) {
    use OrchestratorEvent::*;
    match event {
        TaskCreated { task_id, title, role } => {
            (
                format!("[Task Created] {}: {} (Role: {})\n", task_id, title, role),
                Color::Green
            )
        }
        TaskAssigned { task_id, agent_id, timestamp } => {
            (
                format!("[Task Assigned] {} to {} at {}\n", task_id, agent_id, timestamp.format("%H:%M:%S")),
                Color::Blue
            )
        }
        AgentStarted { task_id, agent_id, timestamp } => {
            (
                format!("[Agent Started] {} by {} at {}\n", task_id, agent_id, timestamp.format("%H:%M:%S")),
                Color::Cyan
            )
        }
        AgentOutput { task_id, agent_id: _, content, timestamp } => {
            let content = content.lines().next().unwrap_or("").trim();
            let content = if content.len() > 50 {
                format!("{}{}", &content[..50], "...")
            } else {
                content.to_string()
            };
            (
                format!("[Agent Output] {}: {} at {}\n", task_id, content, timestamp.format("%H:%M:%S")),
                Color::White
            )
        }
        TaskCompleted { task_id, agent_id, summary, timestamp } => {
            let summary = summary.lines().next().unwrap_or("").trim();
            let summary = if summary.len() > 50 {
                format!("{}{}", &summary[..50], "...")
            } else {
                summary.to_string()
            };
            (
                format!("[Task Completed] {} by {}: {} at {}\n", task_id, agent_id, summary, timestamp.format("%H:%M:%S")),
                Color::Green
            )
        }
        TaskFailed { task_id, agent_id, error, timestamp } => {
            let error = error.lines().next().unwrap_or("").trim();
            let error = if error.len() > 50 {
                format!("{}{}", &error[..50], "...")
            } else {
                error.to_string()
            };
            (
                format!("[Task Failed] {} by {}: {} at {}\n", task_id, agent_id, error, timestamp.format("%H:%M:%S")),
                Color::Red
            )
        }
        WorkflowStarted { workflow_name, total_steps, timestamp } => {
            (
                format!("[Workflow Started] {} with {} steps at {}\n", workflow_name, total_steps, timestamp.format("%H:%M:%S")),
                Color::Yellow
            )
        }
        WorkflowCompleted { workflow_name, timestamp } => {
            (
                format!("[Workflow Completed] {} at {}\n", workflow_name, timestamp.format("%H:%M:%S")),
                Color::Green
            )
        }
    }
}

/// Update task statistics based on the event
fn update_task_stats(event: &OrchestratorEvent, stats: &mut HashMap<String, u32>) {
    use OrchestratorEvent::*;
    match event {
        TaskCreated { .. } => {
            *stats.entry("Task Created".to_string()).or_insert(0) += 1;
        }
        TaskAssigned { .. } => {
            *stats.entry("Task Assigned".to_string()).or_insert(0) += 1;
        }
        AgentStarted { .. } => {
            *stats.entry("Agent Started".to_string()).or_insert(0) += 1;
        }
        AgentOutput { .. } => {
            *stats.entry("Agent Output".to_string()).or_insert(0) += 1;
        }
        TaskCompleted { .. } => {
            *stats.entry("Task Completed".to_string()).or_insert(0) += 1;
        }
        TaskFailed { .. } => {
            *stats.entry("Task Failed".to_string()).or_insert(0) += 1;
        }
        WorkflowStarted { .. } => {
            *stats.entry("Workflow Started".to_string()).or_insert(0) += 1;
        }
        WorkflowCompleted { .. } => {
            *stats.entry("Workflow Completed".to_string()).or_insert(0) += 1;
        }
    }
}

/// Get event type as string
fn get_event_type(event: &OrchestratorEvent) -> String {
    use OrchestratorEvent::*;
    match event {
        TaskCreated { .. } => "Task Created".to_string(),
        TaskAssigned { .. } => "Task Assigned".to_string(),
        AgentStarted { .. } => "Agent Started".to_string(),
        AgentOutput { .. } => "Agent Output".to_string(),
        TaskCompleted { .. } => "Task Completed".to_string(),
        TaskFailed { .. } => "Task Failed".to_string(),
        WorkflowStarted { .. } => "Workflow Started".to_string(),
        WorkflowCompleted { .. } => "Workflow Completed".to_string(),
    }
}

/// Format event details for detailed view
fn format_event_details(event: &OrchestratorEvent) -> String {
    use OrchestratorEvent::*;
    match event {
        TaskCreated { task_id, title, role } => {
            format!(
                "Task ID: {}\nTitle: {}\nRole: {}",
                task_id,
                title,
                role
            )
        }
        TaskAssigned { task_id, agent_id, timestamp } => {
            format!(
                "Task ID: {}\nAgent ID: {}\nTimestamp: {}",
                task_id,
                agent_id,
                timestamp.format("%Y-%m-%d %H:%M:%S")
            )
        }
        AgentStarted { task_id, agent_id, timestamp } => {
            format!(
                "Task ID: {}\nAgent ID: {}\nTimestamp: {}",
                task_id,
                agent_id,
                timestamp.format("%Y-%m-%d %H:%M:%S")
            )
        }
        AgentOutput { task_id, agent_id, content, timestamp } => {
            format!(
                "Task ID: {}\nAgent ID: {}\nTimestamp: {}\nContent: {}",
                task_id,
                agent_id,
                timestamp.format("%Y-%m-%d %H:%M:%S"),
                content
            )
        }
        TaskCompleted { task_id, agent_id, summary, timestamp } => {
            format!(
                "Task ID: {}\nAgent ID: {}\nTimestamp: {}\nSummary: {}",
                task_id,
                agent_id,
                timestamp.format("%Y-%m-%d %H:%M:%S"),
                summary
            )
        }
        TaskFailed { task_id, agent_id, error, timestamp } => {
            format!(
                "Task ID: {}\nAgent ID: {}\nTimestamp: {}\nError: {}",
                task_id,
                agent_id,
                timestamp.format("%Y-%m-%d %H:%M:%S"),
                error
            )
        }
        WorkflowStarted { workflow_name, total_steps, timestamp } => {
            format!(
                "Workflow Name: {}\nTotal Steps: {}\nTimestamp: {}",
                workflow_name,
                total_steps,
                timestamp.format("%Y-%m-%d %H:%M:%S")
            )
        }
        WorkflowCompleted { workflow_name, timestamp } => {
            format!(
                "Workflow Name: {}\nTimestamp: {}",
                workflow_name,
                timestamp.format("%Y-%m-%d %H:%M:%S")
            )
        }
    }
}
