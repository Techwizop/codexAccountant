#![deny(clippy::print_stdout, clippy::print_stderr)]

use std::fmt::Display;

use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use thiserror::Error;

mod in_memory;

pub use crate::in_memory::InMemoryTenancyService;

pub type FirmId = String;
pub type CompanyId = String;
pub type UserId = String;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TenancySnapshot {
    pub firms: Vec<Firm>,
    pub companies: Vec<Company>,
    pub users: Vec<UserAccount>,
}

pub type TenancyResult<T> = Result<T, TenancyError>;

#[derive(Debug, Error)]
pub enum TenancyError {
    #[error("resource not found: {0}")]
    NotFound(String),
    #[error("resource already exists: {0}")]
    Conflict(String),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("operation rejected: {0}")]
    Rejected(String),
    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Firm {
    pub id: FirmId,
    pub name: String,
    pub metadata: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateFirmRequest {
    pub name: String,
    pub metadata: Option<String>,
}

impl CreateFirmRequest {
    pub fn normalize(mut self) -> Result<Self, TenancyError> {
        if self.name.trim().is_empty() {
            return Err(TenancyError::Validation("firm name cannot be empty".into()));
        }
        self.name = self.name.trim().to_string();
        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompanyStatus {
    Active,
    Archived,
}

impl Display for CompanyStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompanyStatus::Active => write!(f, "active"),
            CompanyStatus::Archived => write!(f, "archived"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Company {
    pub id: CompanyId,
    pub firm_id: FirmId,
    pub name: String,
    pub status: CompanyStatus,
    pub base_currency: String,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateCompanyRequest {
    pub firm_id: FirmId,
    pub name: String,
    pub base_currency: String,
    pub tags: Vec<String>,
    pub metadata: Option<String>,
}

impl CreateCompanyRequest {
    pub fn normalize(mut self) -> Result<Self, TenancyError> {
        if self.name.trim().is_empty() {
            return Err(TenancyError::Validation(
                "company name cannot be empty".into(),
            ));
        }
        if self.base_currency.trim().is_empty() {
            return Err(TenancyError::Validation(
                "base currency cannot be empty".into(),
            ));
        }

        self.name = self.name.trim().to_string();
        let code = self.base_currency.trim().to_ascii_uppercase();
        if code.len() != 3 {
            return Err(TenancyError::Validation(
                "base currency must be a 3-letter ISO code".into(),
            ));
        }
        self.base_currency = code;

        let mut seen = std::collections::HashSet::new();
        self.tags = self
            .tags
            .into_iter()
            .map(|tag| tag.trim().to_string())
            .filter(|tag| !tag.is_empty())
            .filter(|tag| seen.insert(tag.to_ascii_lowercase()))
            .collect();

        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    Partner,
    Senior,
    Staff,
    Auditor,
}

impl Role {
    #[must_use]
    pub fn can_manage_companies(self) -> bool {
        matches!(self, Role::Partner | Role::Senior)
    }

    #[must_use]
    pub fn can_post_journal_entries(self) -> bool {
        matches!(self, Role::Partner | Role::Senior | Role::Staff)
    }

    #[must_use]
    pub fn read_only(self) -> bool {
        matches!(self, Role::Auditor)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RoleScope {
    FirmWide,
    Company(CompanyId),
}

impl RoleScope {
    #[must_use]
    pub fn is_firm_wide(&self) -> bool {
        matches!(self, RoleScope::FirmWide)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RoleAssignment {
    pub role: Role,
    pub scope: RoleScope,
}

impl RoleAssignment {
    #[must_use]
    pub fn firm(role: Role) -> Self {
        Self {
            role,
            scope: RoleScope::FirmWide,
        }
    }

    #[must_use]
    pub fn company(role: Role, company_id: CompanyId) -> Self {
        Self {
            role,
            scope: RoleScope::Company(company_id),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserStatus {
    Invited,
    Active,
    Suspended,
    Disabled,
}

impl UserStatus {
    #[must_use]
    pub fn is_active(self) -> bool {
        matches!(self, UserStatus::Active)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleAssignments(pub Vec<RoleAssignment>);

impl RoleAssignments {
    pub fn normalize(self) -> Result<Vec<RoleAssignment>, TenancyError> {
        if self.0.is_empty() {
            return Err(TenancyError::Validation(
                "at least one role assignment is required".into(),
            ));
        }

        let mut dedup = std::collections::HashSet::new();
        for assignment in &self.0 {
            if assignment.role.read_only() && self.0.len() > 1 && assignment.scope.is_firm_wide() {
                return Err(TenancyError::Validation(
                    "auditor role cannot be combined with other firm-wide roles".into(),
                ));
            }
            if !dedup.insert(assignment.clone()) {
                return Err(TenancyError::Validation(
                    "duplicate role assignment detected".into(),
                ));
            }
        }

        Ok(self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InviteUserRequest {
    pub firm_id: FirmId,
    pub email: String,
    pub display_name: String,
    pub roles: Vec<RoleAssignment>,
}

impl InviteUserRequest {
    pub fn normalize(mut self) -> Result<Self, TenancyError> {
        if self.email.trim().is_empty() {
            return Err(TenancyError::Validation("email cannot be empty".into()));
        }
        self.email = self.email.trim().to_ascii_lowercase();
        if !self.email.contains('@') {
            return Err(TenancyError::Validation("email must include '@'".into()));
        }

        self.display_name = self.display_name.trim().to_string();
        if self.display_name.is_empty() {
            return Err(TenancyError::Validation(
                "display name cannot be empty".into(),
            ));
        }

        self.roles = RoleAssignments(self.roles).normalize()?;
        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateUserRolesRequest {
    pub firm_id: FirmId,
    pub user_id: UserId,
    pub roles: Vec<RoleAssignment>,
}

impl UpdateUserRolesRequest {
    pub fn normalize(mut self) -> Result<Self, TenancyError> {
        self.roles = RoleAssignments(self.roles).normalize()?;
        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserAccount {
    pub id: UserId,
    pub firm_id: FirmId,
    pub email: String,
    pub display_name: String,
    pub roles: Vec<RoleAssignment>,
    pub status: UserStatus,
    pub invited_at: DateTime<Utc>,
    pub activated_at: Option<DateTime<Utc>>,
}

impl UserAccount {
    #[must_use]
    pub fn has_role(&self, role: Role) -> bool {
        self.roles.iter().any(|assignment| assignment.role == role)
    }
}

#[async_trait]
pub trait TenancyService: Send + Sync {
    async fn create_firm(&self, request: CreateFirmRequest) -> TenancyResult<Firm>;

    async fn list_firms(&self) -> TenancyResult<Vec<Firm>>;

    async fn get_firm(&self, firm_id: &FirmId) -> TenancyResult<Firm>;

    async fn create_company(&self, request: CreateCompanyRequest) -> TenancyResult<Company>;

    async fn list_companies(&self, firm_id: &FirmId) -> TenancyResult<Vec<Company>>;

    async fn get_company(&self, firm_id: &FirmId, company_id: &CompanyId)
    -> TenancyResult<Company>;

    async fn archive_company(
        &self,
        firm_id: &FirmId,
        company_id: &CompanyId,
    ) -> TenancyResult<Company>;

    async fn reactivate_company(
        &self,
        firm_id: &FirmId,
        company_id: &CompanyId,
    ) -> TenancyResult<Company>;

    async fn invite_user(&self, request: InviteUserRequest) -> TenancyResult<UserAccount>;

    async fn list_users(&self, firm_id: &FirmId) -> TenancyResult<Vec<UserAccount>>;

    async fn get_user(&self, firm_id: &FirmId, user_id: &UserId) -> TenancyResult<UserAccount>;

    async fn set_user_roles(&self, request: UpdateUserRolesRequest) -> TenancyResult<UserAccount>;

    async fn update_user_status(
        &self,
        firm_id: &FirmId,
        user_id: &UserId,
        status: UserStatus,
    ) -> TenancyResult<UserAccount>;
}

fn normalize_company_name(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn company_status_display() {
        assert_eq!(CompanyStatus::Active.to_string(), "active");
        assert_eq!(CompanyStatus::Archived.to_string(), "archived");
    }

    #[test]
    fn create_request_normalizes() {
        let result = CreateCompanyRequest {
            firm_id: "firm-1".into(),
            name: "  Example Firm  ".into(),
            base_currency: " usd ".into(),
            tags: vec![
                "  Retail  ".into(),
                "retail".into(),
                "".into(),
                " Q1 ".into(),
            ],
            metadata: Some("meta".into()),
        }
        .normalize()
        .expect("request should normalize");

        assert_eq!(result.name, "Example Firm");
        assert_eq!(result.base_currency, "USD");
        assert_eq!(
            result.tags,
            vec![String::from("Retail"), String::from("Q1")]
        );
    }

    #[test]
    fn create_request_rejects_invalid_currency() {
        let err = CreateCompanyRequest {
            firm_id: "firm-1".into(),
            name: "Name".into(),
            base_currency: "US".into(),
            tags: vec![],
            metadata: None,
        }
        .normalize()
        .unwrap_err();

        assert!(matches!(err, TenancyError::Validation(_)));
    }

    #[test]
    fn create_firm_request_normalizes() {
        let result = CreateFirmRequest {
            name: "  Demo Firm  ".into(),
            metadata: Some("meta".into()),
        }
        .normalize()
        .expect("firm request should normalize");

        assert_eq!(result.name, "Demo Firm");
    }

    #[test]
    fn invite_user_request_normalizes() {
        let result = InviteUserRequest {
            firm_id: "firm-1".into(),
            email: " USER@example.com ".into(),
            display_name: "  Example User  ".into(),
            roles: vec![RoleAssignment::firm(Role::Partner)],
        }
        .normalize()
        .expect("invite should normalize");

        assert_eq!(result.email, "user@example.com");
        assert_eq!(result.display_name, "Example User");
    }

    #[test]
    fn invite_user_request_rejects_duplicates() {
        let err = InviteUserRequest {
            firm_id: "firm-1".into(),
            email: "user@example.com".into(),
            display_name: "Example User".into(),
            roles: vec![
                RoleAssignment::firm(Role::Partner),
                RoleAssignment::firm(Role::Partner),
            ],
        }
        .normalize()
        .unwrap_err();

        assert!(matches!(err, TenancyError::Validation(_)));
    }
}
