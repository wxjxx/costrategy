use crate::auth::SessionStore;
use crate::dingtalk::DingTalkClient;
use crate::users::UserRepository;

#[derive(Debug, Clone)]
pub struct AppState<C, R> {
    pub dingtalk: C,
    pub users: R,
    pub sessions: SessionStore,
}

impl<C, R> AppState<C, R>
where
    C: DingTalkClient,
    R: UserRepository,
{
    pub fn new(dingtalk: C, users: R, sessions: SessionStore) -> Self {
        Self {
            dingtalk,
            users,
            sessions,
        }
    }
}
