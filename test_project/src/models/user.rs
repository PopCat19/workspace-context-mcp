use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// User model representing a system user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: u64,
    pub username: String,
    pub email: String,
    pub profile: UserProfile,
}

/// User profile information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub first_name: String,
    pub last_name: String,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
}

/// User role enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserRole {
    Admin,
    Moderator,
    User,
    Guest,
}

/// User permissions trait
pub trait UserPermissions {
    fn can_read(&self) -> bool;
    fn can_write(&self) -> bool;
    fn can_delete(&self) -> bool;
}

impl User {
    /// Create a new user
    pub fn new(id: u64, username: String, email: String) -> Self {
        Self {
            id,
            username,
            email,
            profile: UserProfile::default(),
        }
    }

    /// Get user's full name
    pub fn full_name(&self) -> String {
        format!("{} {}", self.profile.first_name, self.profile.last_name)
    }

    /// Update user profile
    pub fn update_profile(&mut self, profile: UserProfile) {
        self.profile = profile;
    }

    /// Check if user is active
    pub fn is_active(&self) -> bool {
        !self.username.is_empty()
    }
}

impl Default for UserProfile {
    fn default() -> Self {
        Self {
            first_name: String::new(),
            last_name: String::new(),
            bio: None,
            avatar_url: None,
        }
    }
}

impl UserPermissions for User {
    fn can_read(&self) -> bool {
        true
    }

    fn can_write(&self) -> bool {
        self.is_active()
    }

    fn can_delete(&self) -> bool {
        false
    }
}

/// User repository for database operations
pub struct UserRepository {
    users: HashMap<u64, User>,
}

impl UserRepository {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
        }
    }

    pub fn add_user(&mut self, user: User) {
        self.users.insert(user.id, user);
    }

    pub fn get_user(&self, id: u64) -> Option<&User> {
        self.users.get(&id)
    }

    pub fn remove_user(&mut self, id: u64) -> Option<User> {
        self.users.remove(&id)
    }
}

/// Constants for user validation
pub const MIN_USERNAME_LENGTH: usize = 3;
pub const MAX_USERNAME_LENGTH: usize = 50;
pub const EMAIL_REGEX: &str = r"^[^\s@]+@[^\s@]+\.[^\s@]+$";

/// Static user count
static mut USER_COUNT: u64 = 0;

/// Get next user ID
pub fn get_next_user_id() -> u64 {
    unsafe {
        USER_COUNT += 1;
        USER_COUNT
    }
}
