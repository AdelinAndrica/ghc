#[repr(i32)]
#[derive(Copy, Clone, Debug)]
pub enum ExitCode {
    Ok = 0,

    // CLI / usage
    Usage = 2,

    // Auth / GitHub API
    Auth = 10,
    GithubApi = 11,

    // Git clone failures
    GitNotFound = 20,
    GitCloneFailed = 21,
}

impl ExitCode {
    pub fn as_i32(self) -> i32 {
        self as i32
    }
}
