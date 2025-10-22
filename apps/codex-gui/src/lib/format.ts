import type { LedgerCurrency } from '../types/protocol'

/**
 * Format currency from minor units (e.g., cents) to display format
 */
export function formatCurrency(amountMinor: number | bigint, currency: LedgerCurrency): string {
  const divisor = Math.pow(10, currency.precision)
  const major = Number(amountMinor) / divisor

  return new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency: currency.code,
    minimumFractionDigits: currency.precision,
    maximumFractionDigits: currency.precision,
  }).format(major)
}

/**
 * Format date from ISO string to localized date
 */
export function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleDateString('en-US', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  })
}

/**
 * Format date and time from ISO string
 */
export function formatDateTime(dateStr: string): string {
  return new Date(dateStr).toLocaleString('en-US', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  })
}

/**
 * Format account type for display
 */
export function formatAccountType(type: string | undefined | null): string {
  if (!type) return 'Unknown'

  return type
    .split(/(?=[A-Z])/)
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(' ')
}

/**
 * Get color class for account type
 */
export function getAccountTypeColor(type: string | undefined | null): string {
  if (!type) return 'text-gray-600 bg-gray-50'

  const colors: Record<string, string> = {
    asset: 'text-green-600 bg-green-50',
    liability: 'text-red-600 bg-red-50',
    equity: 'text-blue-600 bg-blue-50',
    revenue: 'text-purple-600 bg-purple-50',
    expense: 'text-orange-600 bg-orange-50',
    offBalance: 'text-gray-600 bg-gray-50',
  }
  return colors[type.toLowerCase()] || 'text-gray-600 bg-gray-50'
}

/**
 * Get color class for confidence score
 */
export function getConfidenceColor(confidence: number): string {
  if (confidence >= 0.9) return 'text-green-600 bg-green-50'
  if (confidence >= 0.7) return 'text-yellow-600 bg-yellow-50'
  return 'text-red-600 bg-red-50'
}

/**
 * Format confidence as percentage
 */
export function formatConfidence(confidence: number): string {
  return `${(confidence * 100).toFixed(0)}%`
}
