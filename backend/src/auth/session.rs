use crate::auth::CurrentUser;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub const SESSION_COOKIE_NAME: &str = "costrategy_session";

#[derive(Debug, Clone, Default)]
pub struct SessionStore {
    sessions: Arc<Mutex<HashMap<String, CurrentUser>>>,
}

impl SessionStore {
    pub fn create(&self, user: CurrentUser) -> String {
        let token = Uuid::new_v4().to_string();
        self.sessions
            .lock()
            .expect("session store lock")
            .insert(token.clone(), user);
        token
    }

    pub fn get(&self, token: &str) -> Option<CurrentUser> {
        self.sessions
            .lock()
            .expect("session store lock")
            .get(token)
            .cloned()
    }
}
