use std::collections::HashMap;

use chrono::Utc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::Company;
use crate::CompanyId;
use crate::CompanyStatus;
use crate::CreateCompanyRequest;
use crate::CreateFirmRequest;
use crate::Firm;
use crate::FirmId;
use crate::InviteUserRequest;
use crate::TenancyError;
use crate::TenancyResult;
use crate::TenancyService;
use crate::TenancySnapshot;
use crate::UpdateUserRolesRequest;
use crate::UserAccount;
use crate::UserId;
use crate::UserStatus;
use crate::normalize_company_name;

#[derive(Default)]
struct TenancyState {
    firms: HashMap<FirmId, Firm>,
    companies: HashMap<CompanyId, Company>,
    users: HashMap<UserId, UserAccount>,
}

pub struct InMemoryTenancyService {
    state: RwLock<TenancyState>,
}

impl InMemoryTenancyService {
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: RwLock::new(TenancyState::default()),
        }
    }

    #[must_use]
    pub fn from_companies(companies: Vec<Company>) -> Self {
        Self::from_snapshot(TenancySnapshot {
            companies,
            ..TenancySnapshot::default()
        })
    }

    #[must_use]
    pub fn from_snapshot(snapshot: TenancySnapshot) -> Self {
        let mut firms = HashMap::new();
        for firm in snapshot.firms {
            firms.insert(firm.id.clone(), firm);
        }

        let mut companies = HashMap::new();
        for company in snapshot.companies {
            companies.insert(company.id.clone(), company);
        }

        let mut users = HashMap::new();
        for user in snapshot.users {
            users.insert(user.id.clone(), user);
        }

        Self {
            state: RwLock::new(TenancyState {
                firms,
                companies,
                users,
            }),
        }
    }

    pub async fn export_companies(&self) -> Vec<Company> {
        self.export_snapshot().await.companies
    }

    pub async fn export_snapshot(&self) -> TenancySnapshot {
        let guard = self.state.read().await;
        TenancySnapshot {
            firms: guard.firms.values().cloned().collect(),
            companies: guard.companies.values().cloned().collect(),
            users: guard.users.values().cloned().collect(),
        }
    }

    fn generate_company_id() -> CompanyId {
        Uuid::new_v4().to_string()
    }

    fn generate_firm_id() -> FirmId {
        Uuid::new_v4().to_string()
    }

    fn generate_user_id() -> UserId {
        Uuid::new_v4().to_string()
    }

    fn ensure_unique_name(
        state: &TenancyState,
        request: &CreateCompanyRequest,
    ) -> TenancyResult<()> {
        let normalized = normalize_company_name(&request.name);
        let conflict = state.companies.values().any(|company| {
            company.firm_id == request.firm_id
                && company.status == CompanyStatus::Active
                && normalize_company_name(&company.name) == normalized
        });
        if conflict {
            return Err(TenancyError::Conflict(format!(
                "company {} already exists for firm {}",
                request.name, request.firm_id
            )));
        }
        Ok(())
    }

    fn ensure_unique_firm_name(state: &TenancyState, name: &str) -> TenancyResult<()> {
        let normalized = normalize_company_name(name);
        if state
            .firms
            .values()
            .any(|firm| normalize_company_name(&firm.name) == normalized)
        {
            return Err(TenancyError::Conflict(format!(
                "firm {name} already exists"
            )));
        }
        Ok(())
    }

    fn require_firm<'a>(firm_id: &FirmId, state: &'a TenancyState) -> TenancyResult<&'a Firm> {
        state
            .firms
            .get(firm_id)
            .ok_or_else(|| TenancyError::NotFound(format!("firm {firm_id}")))
    }

    fn map_err_not_found(company_id: &str) -> TenancyError {
        TenancyError::NotFound(format!("company {company_id}"))
    }

    fn ensure_unique_user_email(
        state: &TenancyState,
        firm_id: &FirmId,
        email: &str,
    ) -> TenancyResult<()> {
        let normalized = email.to_ascii_lowercase();
        if state
            .users
            .values()
            .any(|user| user.firm_id == *firm_id && user.email.to_ascii_lowercase() == normalized)
        {
            return Err(TenancyError::Conflict(format!(
                "user {email} already exists for firm {firm_id}"
            )));
        }
        Ok(())
    }

    fn map_err_user_not_found(user_id: &str) -> TenancyError {
        TenancyError::NotFound(format!("user {user_id}"))
    }

    fn ensure_user_firm<'a>(
        firm_id: &FirmId,
        user: &'a UserAccount,
        user_id: &str,
    ) -> TenancyResult<&'a UserAccount> {
        if user.firm_id != *firm_id {
            return Err(TenancyError::Rejected(format!(
                "user {user_id} belongs to firm {}, not {firm_id}",
                user.firm_id
            )));
        }
        Ok(user)
    }

    fn ensure_company_firm<'a>(
        firm_id: &FirmId,
        company: &'a Company,
        company_id: &str,
    ) -> TenancyResult<&'a Company> {
        if company.firm_id != *firm_id {
            return Err(TenancyError::Rejected(format!(
                "company {company_id} belongs to firm {}, not {firm_id}",
                company.firm_id
            )));
        }
        Ok(company)
    }
}

impl Default for InMemoryTenancyService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl TenancyService for InMemoryTenancyService {
    async fn create_firm(&self, request: CreateFirmRequest) -> TenancyResult<Firm> {
        let normalized = request.normalize()?;
        let mut guard = self.state.write().await;
        Self::ensure_unique_firm_name(&guard, &normalized.name)?;
        let firm = Firm {
            id: Self::generate_firm_id(),
            name: normalized.name,
            metadata: normalized.metadata,
            created_at: Utc::now(),
        };
        guard.firms.insert(firm.id.clone(), firm.clone());
        Ok(firm)
    }

    async fn list_firms(&self) -> TenancyResult<Vec<Firm>> {
        let guard = self.state.read().await;
        let mut firms: Vec<_> = guard.firms.values().cloned().collect();
        firms.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(firms)
    }

    async fn get_firm(&self, firm_id: &FirmId) -> TenancyResult<Firm> {
        let guard = self.state.read().await;
        let firm = Self::require_firm(firm_id, &guard)?;
        Ok(firm.clone())
    }

    async fn create_company(&self, request: CreateCompanyRequest) -> TenancyResult<Company> {
        let normalized = request.normalize()?;
        let mut guard = self.state.write().await;
        Self::require_firm(&normalized.firm_id, &guard)?;
        Self::ensure_unique_name(&guard, &normalized)?;
        let company = Company {
            id: Self::generate_company_id(),
            firm_id: normalized.firm_id.clone(),
            name: normalized.name.clone(),
            status: CompanyStatus::Active,
            base_currency: normalized.base_currency.clone(),
            tags: normalized.tags.clone(),
            created_at: Utc::now(),
            archived_at: None,
            metadata: normalized.metadata.clone(),
        };
        guard.companies.insert(company.id.clone(), company.clone());
        Ok(company)
    }

    async fn list_companies(&self, firm_id: &FirmId) -> TenancyResult<Vec<Company>> {
        let guard = self.state.read().await;
        let mut companies: Vec<_> = guard
            .companies
            .values()
            .filter(|company| company.firm_id == *firm_id)
            .cloned()
            .collect();
        companies.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(companies)
    }

    async fn get_company(
        &self,
        firm_id: &FirmId,
        company_id: &CompanyId,
    ) -> TenancyResult<Company> {
        let guard = self.state.read().await;
        let company = guard
            .companies
            .get(company_id)
            .ok_or_else(|| Self::map_err_not_found(company_id))?;
        Self::ensure_company_firm(firm_id, company, company_id)?;
        Ok(company.clone())
    }

    async fn archive_company(
        &self,
        firm_id: &FirmId,
        company_id: &CompanyId,
    ) -> TenancyResult<Company> {
        let mut guard = self.state.write().await;
        let company = guard
            .companies
            .get_mut(company_id)
            .ok_or_else(|| Self::map_err_not_found(company_id))?;
        Self::ensure_company_firm(firm_id, company, company_id)?;
        if company.status == CompanyStatus::Archived {
            return Err(TenancyError::Rejected(format!(
                "company {company_id} is already archived"
            )));
        }
        company.status = CompanyStatus::Archived;
        company.archived_at = Some(Utc::now());
        Ok(company.clone())
    }

    async fn reactivate_company(
        &self,
        firm_id: &FirmId,
        company_id: &CompanyId,
    ) -> TenancyResult<Company> {
        let mut guard = self.state.write().await;
        let company = guard
            .companies
            .get_mut(company_id)
            .ok_or_else(|| Self::map_err_not_found(company_id))?;
        Self::ensure_company_firm(firm_id, company, company_id)?;
        if company.status == CompanyStatus::Active {
            return Err(TenancyError::Rejected(format!(
                "company {company_id} is already active"
            )));
        }
        company.status = CompanyStatus::Active;
        company.archived_at = None;
        Ok(company.clone())
    }

    async fn invite_user(&self, request: InviteUserRequest) -> TenancyResult<UserAccount> {
        let normalized = request.normalize()?;
        let mut guard = self.state.write().await;
        Self::require_firm(&normalized.firm_id, &guard)?;
        Self::ensure_unique_user_email(&guard, &normalized.firm_id, &normalized.email)?;
        let user = UserAccount {
            id: Self::generate_user_id(),
            firm_id: normalized.firm_id.clone(),
            email: normalized.email.clone(),
            display_name: normalized.display_name.clone(),
            roles: normalized.roles.clone(),
            status: UserStatus::Invited,
            invited_at: Utc::now(),
            activated_at: None,
        };
        guard.users.insert(user.id.clone(), user.clone());
        Ok(user)
    }

    async fn list_users(&self, firm_id: &FirmId) -> TenancyResult<Vec<UserAccount>> {
        let guard = self.state.read().await;
        let mut users: Vec<_> = guard
            .users
            .values()
            .filter(|user| user.firm_id == *firm_id)
            .cloned()
            .collect();
        users.sort_by(|left, right| left.display_name.cmp(&right.display_name));
        Ok(users)
    }

    async fn get_user(&self, firm_id: &FirmId, user_id: &UserId) -> TenancyResult<UserAccount> {
        let guard = self.state.read().await;
        let user = guard
            .users
            .get(user_id)
            .ok_or_else(|| Self::map_err_user_not_found(user_id))?;
        Self::ensure_user_firm(firm_id, user, user_id)?;
        Ok(user.clone())
    }

    async fn set_user_roles(&self, request: UpdateUserRolesRequest) -> TenancyResult<UserAccount> {
        let normalized = request.normalize()?;
        let mut guard = self.state.write().await;
        let user = guard
            .users
            .get_mut(&normalized.user_id)
            .ok_or_else(|| Self::map_err_user_not_found(&normalized.user_id))?;
        Self::ensure_user_firm(&normalized.firm_id, user, &normalized.user_id)?;
        user.roles = normalized.roles;
        Ok(user.clone())
    }

    async fn update_user_status(
        &self,
        firm_id: &FirmId,
        user_id: &UserId,
        status: UserStatus,
    ) -> TenancyResult<UserAccount> {
        let mut guard = self.state.write().await;
        let user = guard
            .users
            .get_mut(user_id)
            .ok_or_else(|| Self::map_err_user_not_found(user_id))?;
        Self::ensure_user_firm(firm_id, user, user_id)?;
        user.status = status;
        if status.is_active() && user.activated_at.is_none() {
            user.activated_at = Some(Utc::now());
        }
        Ok(user.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Role;
    use crate::RoleAssignment;
    use pretty_assertions::assert_eq;

    async fn create_firm(service: &InMemoryTenancyService, name: &str) -> Firm {
        service
            .create_firm(CreateFirmRequest {
                name: name.into(),
                metadata: None,
            })
            .await
            .expect("create firm")
    }

    async fn create_company(service: &InMemoryTenancyService, firm: &Firm, name: &str) -> Company {
        service
            .create_company(CreateCompanyRequest {
                firm_id: firm.id.clone(),
                name: name.into(),
                base_currency: "usd".into(),
                tags: vec![],
                metadata: None,
            })
            .await
            .expect("create company")
    }

    async fn invite_partner(
        service: &InMemoryTenancyService,
        firm: &Firm,
        email: &str,
    ) -> UserAccount {
        service
            .invite_user(InviteUserRequest {
                firm_id: firm.id.clone(),
                email: email.into(),
                display_name: "Example User".into(),
                roles: vec![RoleAssignment::firm(Role::Partner)],
            })
            .await
            .expect("invite user")
    }

    #[tokio::test]
    async fn creates_and_lists_firms() {
        let service = InMemoryTenancyService::new();
        let _alpha = create_firm(&service, "Alpha LLP").await;
        let beta = create_firm(&service, "Beta LLC").await;

        let firms = service.list_firms().await.expect("list firms");
        assert_eq!(firms.len(), 2);
        assert_eq!(firms[0].name, "Alpha LLP");
        assert_eq!(firms[1].id, beta.id);
    }

    #[tokio::test]
    async fn prevents_duplicate_firm_names() {
        let service = InMemoryTenancyService::new();
        let _ = create_firm(&service, "Acme Firm").await;
        let err = service
            .create_firm(CreateFirmRequest {
                name: "acme firm".into(),
                metadata: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, TenancyError::Conflict(_)));
    }

    #[tokio::test]
    async fn creates_and_lists_companies() {
        let service = InMemoryTenancyService::new();
        let firm = create_firm(&service, "Demo Firm").await;
        let created = create_company(&service, &firm, "Demo Co").await;

        assert_eq!(created.name, "Demo Co");
        assert_eq!(created.base_currency, "USD");
        assert_eq!(created.status, CompanyStatus::Active);

        let listed = service
            .list_companies(&firm.id)
            .await
            .expect("list companies");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0], created);
    }

    #[tokio::test]
    async fn prevents_duplicate_company_names() {
        let service = InMemoryTenancyService::new();
        let firm = create_firm(&service, "Demo Firm").await;
        let _ = create_company(&service, &firm, "Demo Co").await;

        let err = service
            .create_company(CreateCompanyRequest {
                firm_id: firm.id.clone(),
                name: " demo co ".into(),
                base_currency: "usd".into(),
                tags: vec![],
                metadata: None,
            })
            .await
            .unwrap_err();

        assert!(matches!(err, TenancyError::Conflict(_)));
    }

    #[tokio::test]
    async fn archives_and_reactivates_company() {
        let service = InMemoryTenancyService::new();
        let firm = create_firm(&service, "Demo Firm").await;
        let created = create_company(&service, &firm, "Demo Co").await;

        let archived = service
            .archive_company(&firm.id, &created.id)
            .await
            .expect("archive company");
        assert_eq!(archived.status, CompanyStatus::Archived);
        assert!(archived.archived_at.is_some());

        let reactivated = service
            .reactivate_company(&firm.id, &created.id)
            .await
            .expect("reactivate company");
        assert_eq!(reactivated.status, CompanyStatus::Active);
        assert!(reactivated.archived_at.is_none());
    }

    #[tokio::test]
    async fn rejects_cross_firm_operations() {
        let service = InMemoryTenancyService::new();
        let firm_a = create_firm(&service, "Firm A").await;
        let firm_b = create_firm(&service, "Firm B").await;
        let company = create_company(&service, &firm_a, "Demo Co").await;

        let err = service
            .archive_company(&firm_b.id, &company.id)
            .await
            .unwrap_err();
        assert!(matches!(err, TenancyError::Rejected(_)));
    }

    #[tokio::test]
    async fn invites_users_and_lists_by_firm() {
        let service = InMemoryTenancyService::new();
        let firm = create_firm(&service, "Demo Firm").await;
        let user = invite_partner(&service, &firm, "user@example.com").await;

        assert_eq!(user.status, UserStatus::Invited);
        let users = service.list_users(&firm.id).await.expect("list users");
        assert_eq!(users.len(), 1);
        assert_eq!(users[0], user);
    }

    #[tokio::test]
    async fn prevents_duplicate_user_email() {
        let service = InMemoryTenancyService::new();
        let firm = create_firm(&service, "Demo Firm").await;
        let _ = invite_partner(&service, &firm, "user@example.com").await;
        let err = service
            .invite_user(InviteUserRequest {
                firm_id: firm.id.clone(),
                email: "USER@example.com".into(),
                display_name: "Other User".into(),
                roles: vec![RoleAssignment::firm(Role::Partner)],
            })
            .await
            .unwrap_err();
        assert!(matches!(err, TenancyError::Conflict(_)));
    }

    #[tokio::test]
    async fn prevents_cross_firm_user_access() {
        let service = InMemoryTenancyService::new();
        let firm_a = create_firm(&service, "Firm A").await;
        let firm_b = create_firm(&service, "Firm B").await;
        let user = invite_partner(&service, &firm_a, "user@example.com").await;

        let err = service.get_user(&firm_b.id, &user.id).await.unwrap_err();
        assert!(matches!(err, TenancyError::Rejected(_)));
    }

    #[tokio::test]
    async fn set_user_roles_updates_assignments() {
        let service = InMemoryTenancyService::new();
        let firm = create_firm(&service, "Demo Firm").await;
        let company = create_company(&service, &firm, "Demo Co").await;
        let user = invite_partner(&service, &firm, "user@example.com").await;

        let updated = service
            .set_user_roles(UpdateUserRolesRequest {
                firm_id: firm.id.clone(),
                user_id: user.id.clone(),
                roles: vec![
                    RoleAssignment::firm(Role::Senior),
                    RoleAssignment::company(Role::Staff, company.id.clone()),
                ],
            })
            .await
            .expect("set user roles");

        assert!(updated.has_role(Role::Senior));
        assert_eq!(updated.roles.len(), 2);
    }

    #[tokio::test]
    async fn update_user_status_tracks_activation() {
        let service = InMemoryTenancyService::new();
        let firm = create_firm(&service, "Demo Firm").await;
        let user = invite_partner(&service, &firm, "user@example.com").await;

        let activated = service
            .update_user_status(&firm.id, &user.id, UserStatus::Active)
            .await
            .expect("activate user");
        assert!(activated.status.is_active());
        assert!(activated.activated_at.is_some());
    }

    #[tokio::test]
    async fn round_trips_snapshot() {
        let service = InMemoryTenancyService::new();
        let firm = create_firm(&service, "Demo Firm").await;
        let company = create_company(&service, &firm, "Snapshot Co").await;
        let user = invite_partner(&service, &firm, "user@example.com").await;

        let snapshot = service.export_snapshot().await;
        assert_eq!(snapshot.firms.len(), 1);
        assert_eq!(snapshot.companies[0], company);
        assert_eq!(snapshot.users[0].id, user.id);

        let restored = InMemoryTenancyService::from_snapshot(snapshot.clone());
        let round_trip = restored.export_snapshot().await;
        assert_eq!(round_trip.firms, snapshot.firms);
        assert_eq!(round_trip.companies, snapshot.companies);
        assert_eq!(round_trip.users, snapshot.users);
    }
}
