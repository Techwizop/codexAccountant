import { useQuery, useMutation, type UseQueryOptions } from '@tanstack/react-query'
import { apiClient } from './client'
import type {
  LedgerListCompaniesParams,
  LedgerListCompaniesResponse,
  LedgerListAccountsParams,
  LedgerListAccountsResponse,
  LedgerListEntriesParams,
  LedgerListEntriesResponse,
  LedgerGetCompanyContextParams,
  LedgerGetCompanyContextResponse,
  LedgerProcessDocumentParams,
  LedgerProcessDocumentResponse,
} from '../types/protocol'

/**
 * Hook to list companies
 */
export function useCompanies(
  params: LedgerListCompaniesParams = { search: null },
  options?: Omit<UseQueryOptions<LedgerListCompaniesResponse>, 'queryKey' | 'queryFn'>
) {
  return useQuery({
    queryKey: ['companies', params],
    queryFn: () =>
      apiClient.call<LedgerListCompaniesParams, LedgerListCompaniesResponse>(
        'ledgerListCompanies',
        params
      ),
    ...options,
  })
}

/**
 * Hook to list accounts for a company
 */
export function useAccounts(
  params: LedgerListAccountsParams,
  options?: Omit<UseQueryOptions<LedgerListAccountsResponse>, 'queryKey' | 'queryFn'>
) {
  return useQuery({
    queryKey: ['accounts', params],
    queryFn: () =>
      apiClient.call<LedgerListAccountsParams, LedgerListAccountsResponse>(
        'ledgerListAccounts',
        params
      ),
    enabled: !!params.companyId,
    ...options,
  })
}

/**
 * Hook to list journal entries for a company
 */
export function useEntries(
  params: LedgerListEntriesParams,
  options?: Omit<UseQueryOptions<LedgerListEntriesResponse>, 'queryKey' | 'queryFn'>
) {
  return useQuery({
    queryKey: ['entries', params],
    queryFn: () =>
      apiClient.call<LedgerListEntriesParams, LedgerListEntriesResponse>(
        'ledgerListEntries',
        params
      ),
    enabled: !!params.companyId,
    ...options,
  })
}

/**
 * Hook to get company context (for AI)
 */
export function useCompanyContext(
  params: LedgerGetCompanyContextParams,
  options?: Omit<UseQueryOptions<LedgerGetCompanyContextResponse>, 'queryKey' | 'queryFn'>
) {
  return useQuery({
    queryKey: ['companyContext', params],
    queryFn: () =>
      apiClient.call<LedgerGetCompanyContextParams, LedgerGetCompanyContextResponse>(
        'ledgerGetCompanyContext',
        params
      ),
    enabled: !!params.companyId,
    ...options,
  })
}

/**
 * Mutation hook to process a document
 */
export function useProcessDocument() {
  return useMutation({
    mutationFn: (params: LedgerProcessDocumentParams) =>
      apiClient.call<LedgerProcessDocumentParams, LedgerProcessDocumentResponse>(
        'ledgerProcessDocument',
        params
      ),
  })
}
