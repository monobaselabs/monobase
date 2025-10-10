import { createFileRoute, Outlet } from '@tanstack/react-router'
import { requireAuth, requirePerson, composeGuards } from '@/utils/guards'
import { AppSidebar, type NavGroup } from '@/components/app-sidebar'
import {
  SidebarProvider,
  SidebarInset,
  SidebarTrigger
} from "@monobase/ui/components/sidebar"
import {
  Home,
  User,
  Shield,
  Bell,
} from 'lucide-react'
import { UserButton } from '@daveyplate/better-auth-ui'
import { useUnreadNotifications } from '@monobase/sdk/react/hooks/use-notifications'

export const Route = createFileRoute('/_dashboard')({
  beforeLoad: composeGuards(requireAuth, requirePerson),
  component: DashboardLayout,
})

function DashboardLayout() {
  // Fetch unread notifications count for badge
  const { data: unreadData } = useUnreadNotifications()
  const unreadCount = unreadData?.pagination?.totalCount || 0

  // Define navigation structure for the admin dashboard (simplified)
  const navGroups: NavGroup[] = [
    {
      label: "Navigation",
      items: [
        {
          title: "Dashboard",
          url: "/dashboard",
          icon: Home,
          badge: null,
        },
        {
          title: "Notifications",
          url: "/notifications",
          icon: Bell,
          badge: unreadCount > 0 ? unreadCount : null,
        },
      ]
    },
    {
      label: "Settings",
      items: [
        {
          title: "Account Settings",
          url: "/settings/account",
          icon: User,
        },
        {
          title: "Security",
          url: "/settings/security",
          icon: Shield,
        },
      ]
    }
  ]

  return (
    <SidebarProvider>
      <AppSidebar
        navGroups={navGroups}
        headerTitle="MONOBASE"
        headerSubtitle="Admin Portal"
      />
      <SidebarInset>
        <header className="flex h-16 shrink-0 items-center gap-2 border-b px-4">
          <SidebarTrigger className="-ml-1" />
          <div className="flex flex-1 items-center gap-2">
            {/* Breadcrumbs will go here */}
          </div>
          <UserButton
            variant="ghost"
            disableDefaultLinks
          />
        </header>
        <main className="flex-1">
          <Outlet />
        </main>
      </SidebarInset>
    </SidebarProvider>
  )
}
