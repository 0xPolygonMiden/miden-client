import Link from "next/link";

const routes = {
  gettingStarted: '/',
  accounts: '/accounts',
  notes: '/notes',
  transactions: '/transactions'
};

const menuItems = [
  {
    name: 'Getting Started',
    href: routes.gettingStarted,
  },
  {
    name: 'Accounts',
    href: routes.accounts,
  },
  {
    name: 'Notes',
    href: routes.notes,
  },
  {
    name: 'Transactions',
    href: routes.transactions,
  }
];

type SidebarProps = {
  className?: string;
};

export default function Sidebar({ className }: SidebarProps) {
  return (
    <aside>
      <nav>
        <ul>
          {menuItems.map((item, index) => (
            <li key={index}>
              <Link href={item.href}>{item.name}</Link>
            </li>
          ))}
        </ul>
      </nav>
    </aside>
  );
}