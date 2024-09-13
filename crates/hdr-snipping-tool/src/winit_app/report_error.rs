pub fn make_message(action: &str) -> String {
    format!("We encountered an error while {action}.\nMore details are in the logs.")
}
