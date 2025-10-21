import { useCompanies } from '@/api/hooks'
import { Button } from '@/components/ui/button'
import { ChevronDown } from 'lucide-react'

interface HeaderProps {
  selectedCompanyId: string | null
  onSelectCompany: (companyId: string | null) => void
}

export function Header({ selectedCompanyId, onSelectCompany }: HeaderProps) {
  const { data: companiesData } = useCompanies()

  const selectedCompany = companiesData?.companies.find((c) => c.id === selectedCompanyId)

  return (
    <header className="flex h-16 items-center justify-between border-b bg-white px-6">
      <div className="flex items-center gap-4">
        <Button
          variant="outline"
          className="gap-2"
          onClick={() => {
            // TODO: Open company selector dropdown
            if (companiesData?.companies.length) {
              // For now, just cycle through companies
              const currentIndex = companiesData.companies.findIndex(
                (c) => c.id === selectedCompanyId
              )
              const nextIndex = (currentIndex + 1) % companiesData.companies.length
              onSelectCompany(companiesData.companies[nextIndex].id)
            }
          }}
        >
          {selectedCompany ? selectedCompany.name : 'Select Company'}
          <ChevronDown className="h-4 w-4" />
        </Button>
      </div>
      <div className="flex items-center gap-4">
        <span className="text-sm text-gray-600">Welcome to Codex Accounting</span>
      </div>
    </header>
  )
}
