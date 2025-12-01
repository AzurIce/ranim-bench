import { cn } from '../lib/utils';

interface NavbarProps {
  activeTab: 'charts' | 'system';
  onTabChange: (tab: 'charts' | 'system') => void;
}

export function Navbar({ activeTab, onTabChange }: NavbarProps) {
  return (
    <nav className="bg-white border-b border-gray-200 px-6 py-4 sticky top-0 z-10 shadow-sm flex justify-between items-center">
      <div className="flex items-center gap-3">
        <div className="bg-blue-600 w-8 h-8 rounded-lg flex items-center justify-center text-white font-bold">R</div>
        <h1 className="text-xl font-bold text-gray-800">Ranim Bench</h1>
      </div>
      <div className="flex gap-4">
        <button
          onClick={() => onTabChange('charts')}
          className={cn("px-4 py-2 rounded-md text-sm font-medium transition-colors",
            activeTab === 'charts' ? "bg-blue-50 text-blue-700" : "text-gray-600 hover:bg-gray-100"
          )}
        >
          Charts
        </button>
        <button
          onClick={() => onTabChange('system')}
          className={cn("px-4 py-2 rounded-md text-sm font-medium transition-colors",
            activeTab === 'system' ? "bg-blue-50 text-blue-700" : "text-gray-600 hover:bg-gray-100"
          )}
        >
          System Info
        </button>
      </div>
    </nav>
  );
}
