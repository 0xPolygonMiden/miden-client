import Sidebar from "./sidebar";

const DefaultLayout = ({ children }: {children: React.ReactNode}) => (
  <div style={{ display: 'flex' }}>
    <Sidebar />
    <main style={{ flexGrow: 1 }}>{children}</main>
  </div>
);

export default DefaultLayout;