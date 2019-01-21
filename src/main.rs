use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::prelude::*;

use failure::Error;

use i3ipc::I3Connection;
use i3ipc::I3EventListener;
use i3ipc::Subscription;
use i3ipc::event::Event;
use i3ipc::reply;

const COLOR_CLASSES: [&'static str; 6] = [
    "#C594C5",
    "#6699CC",
    "#5FB3B3",
    "#99C794",
    "#FAC863",
    "#F99157",
    // "@base0E",
    // "@base0D",
    // "@base0C",
    // "@base0B",
    // "@base0A",
    // "@base09",
];

fn workspace_module_output(
    workspace: &reply::Workspace,
    last_workspace_num: i32,
    workspace_count: i32,
) -> String {
    let mut module_text = workspace.num.to_string();
    if workspace.focused {
        module_text = format!(
            // "<span color=\"@base08\">{}</span>",
            "<span color=\"#EC5F67\">{}</span>",
            module_text,
        );
    }

    let color = COLOR_CLASSES[(workspace.num-1) as usize % COLOR_CLASSES.len()];
    let left_color = COLOR_CLASSES[(last_workspace_num-1) as usize % COLOR_CLASSES.len()];

    if workspace.num >= workspace_count && workspace.num == 1 {
        return format!(
            "<span background=\"{}\"> {} </span><span color=\"{}\"></span>\n",
            color,
            module_text,
            color,
        )
    } else if workspace.num == 1 {
        return format!(
            "<span background=\"{}\"> {}</span>\n",
            color,
            module_text,
        )
    } else if workspace.num >= workspace_count {
        return format!(
            "<span background=\"{}\" color=\"{}\"></span><span background=\"{}\" color=\"{}\"></span><span background=\"{}\"> {} </span><span color=\"{}\"></span>\n",
            left_color,
            left_color,
            left_color,
            color,
            color,
            module_text,
            color,
        )
    } else {
        return format!(
            "<span background=\"{}\" color=\"{}\"></span><span background=\"{}\" color=\"{}\"></span><span background=\"{}\"> {}</span>\n",
            left_color,
            left_color,
            left_color,
            color,
            color,
            module_text,
        )
    }
}

fn refresh_workspaces(
    workspaces: Vec<reply::Workspace>,
    output_dir: &std::path::Path,
    workspace_module_outputs: &mut HashMap<i32, String>,
) -> Result<(), Error> {
    let workspace_count = workspaces.len() as i32;

    let mut latest_workspace_nums: HashSet<i32> = HashSet::new();
    let mut last_workspace_num = 1;
    for workspace in workspaces {
        let module_output = workspace_module_output(
            &workspace,
            last_workspace_num,
            workspace_count,
        );
        let old_module_output = if let Some(old_module_output) = workspace_module_outputs
            .get(&workspace.num) {
            old_module_output
        } else {
            ""
        };

        if module_output != *old_module_output {
            fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(output_dir.join(workspace.num.to_string()))?
                .write_all(module_output.as_bytes())?;
        }
        workspace_module_outputs.insert(workspace.num, module_output);
        latest_workspace_nums.insert(workspace.num);
        last_workspace_num = workspace.num
    }

    for existing_workspace_output in output_dir.read_dir()? {
        let workspace_output_entry = existing_workspace_output?.file_name().into_string();
        let workspace_num: i32 = match workspace_output_entry {
            Ok(s) => s.parse()?,
            Err(_) => {
                fs::remove_dir_all(output_dir)?;
                return Ok(())
            }
        };
        if latest_workspace_nums.contains(&workspace_num) {
            continue
        }

        fs::OpenOptions::new()
            .append(true)
            .open(output_dir.join(workspace_num.to_string()))?
            .write_all("\n\n".as_bytes())?;
        workspace_module_outputs.remove(&workspace_num);
    }

    return Ok(())
}

#[derive(Debug)]
struct WorkspaceOutputManager<'a> {
    wm_connection: I3Connection,
    wm_event_listener: I3EventListener,
    workspace_module_outputs: HashMap<i32, String>,
    output_dir: &'a std::path::Path,
}

impl<'a> WorkspaceOutputManager<'a> {
    fn new(output_dir: &'a std::path::Path) -> Result<WorkspaceOutputManager<'a>, Error> {
        return Ok(WorkspaceOutputManager{
            output_dir: output_dir,
            wm_connection: I3Connection::connect()?,
            wm_event_listener: I3EventListener::connect()?,
            workspace_module_outputs: HashMap::new(),
        });
    }

    fn run(&mut self) -> Result<(), Error> {
        if self.output_dir.exists() {
            fs::remove_dir_all(self.output_dir)?;
        }
        fs::create_dir_all(self.output_dir)?;
        self.wm_event_listener.subscribe(&[Subscription::Workspace])?;

        let workspaces = self.wm_connection.get_workspaces()?.workspaces;
        refresh_workspaces(
            workspaces,
            self.output_dir,
            &mut self.workspace_module_outputs
        )?;

        for event in self.wm_event_listener.listen() {
            match event? {
                Event::WorkspaceEvent(_) => {
                    let workspaces = self.wm_connection.get_workspaces()?.workspaces;
                    refresh_workspaces(
                        workspaces,
                        self.output_dir,
                        &mut self.workspace_module_outputs
                    )?;
                },
                _ => unreachable!(),
            }
        }

        return Ok(())
    }
}

fn main() {
    let output_dir = if let Some(cache_dir) = dirs::cache_dir() {
        cache_dir.join("waybar-sway-workspaces")
    } else {
        eprintln!("No cache dir available for output files");
        std::process::exit(1);
    };

    let mut workspace_output_manager: WorkspaceOutputManager = match WorkspaceOutputManager::new(&output_dir) {
        Ok(workspace_output_manager) => workspace_output_manager,
        Err(e) => {
            eprintln!("Error instantiating workspace output manager: {}", e);
            std::process::exit(1);
        },
    };

    if let Err(e) = workspace_output_manager.run() {
        eprintln!("Error running workspace output manager: {}", e);
        std::process::exit(1);
    };
}
