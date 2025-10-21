use std::sync::Arc;

use codex_tenancy::Company;
use codex_tenancy::CompanyId;
use codex_tenancy::CreateCompanyRequest;
use codex_tenancy::CreateFirmRequest;
use codex_tenancy::Firm;
use codex_tenancy::FirmId;
use codex_tenancy::InviteUserRequest;
use codex_tenancy::TenancyResult;
use codex_tenancy::TenancyService;
use codex_tenancy::UpdateUserRolesRequest;
use codex_tenancy::UserAccount;
use codex_tenancy::UserId;
use codex_tenancy::UserStatus;

#[derive(Clone)]
pub struct TenancyFacade {
    service: Arc<dyn TenancyService>,
}

impl TenancyFacade {
    pub fn new(service: Arc<dyn TenancyService>) -> Self {
        Self { service }
    }

    pub async fn create_firm(&self, request: CreateFirmRequest) -> TenancyResult<Firm> {
        self.service.create_firm(request).await
    }

    pub async fn list_firms(&self) -> TenancyResult<Vec<Firm>> {
        self.service.list_firms().await
    }

    pub async fn get_firm(&self, firm_id: &FirmId) -> TenancyResult<Firm> {
        self.service.get_firm(firm_id).await
    }

    pub async fn create_company(&self, request: CreateCompanyRequest) -> TenancyResult<Company> {
        self.service.create_company(request).await
    }

    pub async fn list_companies(&self, firm_id: &FirmId) -> TenancyResult<Vec<Company>> {
        self.service.list_companies(firm_id).await
    }

    pub async fn get_company(
        &self,
        firm_id: &FirmId,
        company_id: &CompanyId,
    ) -> TenancyResult<Company> {
        self.service.get_company(firm_id, company_id).await
    }

    pub async fn archive_company(
        &self,
        firm_id: &FirmId,
        company_id: &CompanyId,
    ) -> TenancyResult<Company> {
        self.service.archive_company(firm_id, company_id).await
    }

    pub async fn reactivate_company(
        &self,
        firm_id: &FirmId,
        company_id: &CompanyId,
    ) -> TenancyResult<Company> {
        self.service.reactivate_company(firm_id, company_id).await
    }

    pub async fn invite_user(&self, request: InviteUserRequest) -> TenancyResult<UserAccount> {
        self.service.invite_user(request).await
    }

    pub async fn list_users(&self, firm_id: &FirmId) -> TenancyResult<Vec<UserAccount>> {
        self.service.list_users(firm_id).await
    }

    pub async fn get_user(&self, firm_id: &FirmId, user_id: &UserId) -> TenancyResult<UserAccount> {
        self.service.get_user(firm_id, user_id).await
    }

    pub async fn set_user_roles(
        &self,
        request: UpdateUserRolesRequest,
    ) -> TenancyResult<UserAccount> {
        self.service.set_user_roles(request).await
    }

    pub async fn update_user_status(
        &self,
        firm_id: &FirmId,
        user_id: &UserId,
        status: UserStatus,
    ) -> TenancyResult<UserAccount> {
        self.service
            .update_user_status(firm_id, user_id, status)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_tenancy::InMemoryTenancyService;
    use codex_tenancy::Role;
    use codex_tenancy::RoleAssignment;
    use pretty_assertions::assert_eq;

    #[tokio::test]
    async fn facade_round_trips() {
        let service: Arc<dyn TenancyService> = Arc::new(InMemoryTenancyService::new());
        let facade = TenancyFacade::new(service);

        let firm = facade
            .create_firm(CreateFirmRequest {
                name: "Demo Firm".into(),
                metadata: None,
            })
            .await
            .expect("firm should be created");

        let listed_firms = facade.list_firms().await.expect("firms should list");
        assert_eq!(listed_firms.len(), 1);
        assert_eq!(listed_firms[0], firm);

        let created = facade
            .create_company(CreateCompanyRequest {
                firm_id: firm.id.clone(),
                name: "Demo Co".into(),
                base_currency: "usd".into(),
                tags: vec!["pilot".into()],
                metadata: None,
            })
            .await
            .expect("company should be created");

        let fetched = facade
            .get_company(&firm.id, &created.id)
            .await
            .expect("company should be fetched");
        assert_eq!(fetched, created);

        let archived = facade
            .archive_company(&firm.id, &created.id)
            .await
            .expect("company should be archived");
        assert_eq!(archived.status, codex_tenancy::CompanyStatus::Archived);

        let reactivated = facade
            .reactivate_company(&firm.id, &created.id)
            .await
            .expect("company should be reactivated");
        assert_eq!(reactivated.status, codex_tenancy::CompanyStatus::Active);

        let listed = facade
            .list_companies(&firm.id)
            .await
            .expect("companies should list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0], reactivated);

        let user = facade
            .invite_user(InviteUserRequest {
                firm_id: firm.id.clone(),
                email: "user@example.com".into(),
                display_name: "Example User".into(),
                roles: vec![RoleAssignment::firm(Role::Partner)],
            })
            .await
            .expect("user should be invited");

        let fetched_user = facade
            .get_user(&firm.id, &user.id)
            .await
            .expect("user should be fetched");
        assert_eq!(fetched_user, user);

        let listed_users = facade
            .list_users(&firm.id)
            .await
            .expect("users should list");
        assert_eq!(listed_users.len(), 1);
        assert_eq!(listed_users[0], user);

        let updated_user = facade
            .set_user_roles(UpdateUserRolesRequest {
                firm_id: firm.id.clone(),
                user_id: user.id.clone(),
                roles: vec![RoleAssignment::firm(Role::Senior)],
            })
            .await
            .expect("user roles should update");
        assert!(updated_user.has_role(Role::Senior));

        let activated = facade
            .update_user_status(&firm.id, &user.id, UserStatus::Active)
            .await
            .expect("user should activate");
        assert!(activated.status.is_active());
    }
}
