//! Every mutation the UI can request, applied after the page has been drawn.

use crate::widgets::{open_file, show_in_folder};
use crate::App;

pub(crate) enum Action {
    Toggle(u64),
    Pause(u64),
    Resume(u64),
    Cancel(u64),
    Retry(u64),
    Remove(u64),
    Delete(u64),
    OpenFolder(u64),
    OpenFile(u64),
    CopyLink(String),
    PauseAll,
    ResumeAll,
    PauseSelected,
    ResumeSelected,
    RemoveSelected,
    ClearList,
}

pub(crate) fn apply(app: &mut App, action: Action) {
    match action {
        Action::Toggle(id) => {
            if !app.selected.remove(&id) {
                app.selected.insert(id);
            }
        }
        Action::Pause(id) => app.manager.pause(id),
        Action::Resume(id) => app.manager.resume(id),
        Action::Cancel(id) => app.manager.cancel(id),
        Action::Retry(id) => app.manager.retry(id),
        Action::Remove(id) => {
            app.manager.remove(id);
            app.selected.remove(&id);
        }
        Action::Delete(id) => {
            app.manager.remove_with_file(id);
            app.selected.remove(&id);
        }
        Action::OpenFolder(id) => {
            if let Some(job) = app.manager.jobs.iter().find(|job| job.id == id) {
                show_in_folder(&job.path());
            }
        }
        Action::OpenFile(id) => {
            if let Some(job) = app.manager.jobs.iter().find(|job| job.id == id) {
                open_file(&job.path());
            }
        }
        Action::CopyLink(url) => {
            if let Some(clipboard) = app.clipboard.as_mut() {
                let _ = clipboard.set_text(url);
                app.notify("Link copied.", false);
            } else {
                app.notify("The clipboard isn't available.", true);
            }
        }
        Action::PauseAll => app.manager.pause_all(),
        Action::ResumeAll => app.manager.resume_all(),
        Action::PauseSelected => {
            for id in app.selected.iter().copied().collect::<Vec<_>>() {
                app.manager.pause(id);
            }
        }
        Action::ResumeSelected => {
            for id in app.selected.iter().copied().collect::<Vec<_>>() {
                app.manager.resume(id);
            }
        }
        Action::RemoveSelected => {
            let ids: Vec<u64> = app.selected.iter().copied().collect();
            for id in ids {
                app.manager.remove(id);
            }
            app.selected.clear();
        }
        Action::ClearList => {
            app.selected.clear();
            app.manager.clear_finished();
        }
    }
}
