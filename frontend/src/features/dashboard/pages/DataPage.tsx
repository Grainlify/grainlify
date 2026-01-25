import { useState } from 'react';
import { ChevronDown, Info } from 'lucide-react';
import { BarChart, Bar, LineChart, Line as RechartsLine, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, ComposedChart } from 'recharts';
import { ComposableMap, Geographies, Geography, Marker, ZoomableGroup, Line as MapLine } from "react-simple-maps";
import { useTheme } from '../../../shared/contexts/ThemeContext';

export function DataPage() {
  const { theme } = useTheme();
  const [mapZoom, setMapZoom] = useState(1);
  const [mapCenter, setMapCenter] = useState<[number, number]>([0, 0]);

  const geoUrl = "https://cdn.jsdelivr.net/npm/world-atlas@2/countries-110m.json";

  const countryCoordinates: Record<string, [number, number]> = {
    'United Kingdom': [-3.435973, 55.378051],
    'Germany': [10.451526, 51.165691],
    'Canada': [-106.346771, 56.130366],
    'India': [78.96288, 20.593684],
    'Brazil': [-51.92528, -14.235004],
    'Netherlands': [5.291266, 52.132633],
    'Australia': [133.775136, -25.274398],
    'Spain': [-3.74922, 40.463667],
    'Italy': [12.56738, 41.87194],
    'Poland': [19.145136, 51.919438],
    'Sweden': [18.643501, 60.128161],
    'Japan': [138.252924, 36.204824],
    'China': [104.195397, 35.86166],
  };

  const [activeTab, setActiveTab] = useState('overview');
  const [projectInterval, setProjectInterval] = useState('Monthly interval');
  const [contributorInterval, setContributorInterval] = useState('Monthly interval');
  const [showProjectIntervalDropdown, setShowProjectIntervalDropdown] = useState(false);
  const [showContributorIntervalDropdown, setShowContributorIntervalDropdown] = useState(false);
  const [projectFilters, setProjectFilters] = useState({
    new: false,
    reactivated: false,
    active: false,
    churned: false,
    prMerged: false,
  });
  const [contributorFilters, setContributorFilters] = useState({
    new: false,
    reactivated: false,
    active: false,
    churned: false,
    prMerged: false,
  });

  // Sample data for project activity (monthly data)
  const projectActivityData = [
    { month: 'January', value: 45, trend: 40, new: 12, reactivated: 5, active: 28, churned: -8, rewarded: 15420 },
    { month: 'February', value: 38, trend: 42, new: 8, reactivated: 4, active: 26, churned: -6, rewarded: 12300 },
    { month: 'March', value: 52, trend: 45, new: 15, reactivated: 7, active: 30, churned: -5, rewarded: 18650 },
    { month: 'April', value: 48, trend: 50, new: 11, reactivated: 6, active: 31, churned: -7, rewarded: 16800 },
    { month: 'May', value: 58, trend: 52, new: 18, reactivated: 8, active: 32, churned: -4, rewarded: 22100 },
    { month: 'June', value: 55, trend: 55, new: 14, reactivated: 6, active: 35, churned: -9, rewarded: 20500 },
    { month: 'July', value: 42, trend: 54, new: 9, reactivated: 5, active: 28, churned: -10, rewarded: 14200 },
    { month: 'August', value: 48, trend: 50, new: 12, reactivated: 7, active: 29, churned: -6, rewarded: 17300 },
    { month: 'September', value: 62, trend: 52, new: 20, reactivated: 9, active: 33, churned: -5, rewarded: 24800 },
    { month: 'October', value: 58, trend: 58, new: 16, reactivated: 8, active: 34, churned: -7, rewarded: 21900 },
    { month: 'November', value: 45, trend: 56, new: 10, reactivated: 6, active: 29, churned: -8, rewarded: 15600 },
    { month: 'December', value: 52, trend: 52, new: 13, reactivated: 7, active: 32, churned: -10, rewarded: 18900 },
  ];

  // Sample data for contributor activity
  const contributorActivityData = [
    { month: 'January', value: 42, trend: 38, new: 10, reactivated: 4, active: 28, churned: -6, rewarded: 14200 },
    { month: 'February', value: 35, trend: 40, new: 7, reactivated: 3, active: 25, churned: -5, rewarded: 11800 },
    { month: 'March', value: 48, trend: 42, new: 13, reactivated: 6, active: 29, churned: -4, rewarded: 16900 },
    { month: 'April', value: 45, trend: 46, new: 11, reactivated: 5, active: 29, churned: -6, rewarded: 15300 },
    { month: 'May', value: 38, trend: 44, new: 8, reactivated: 4, active: 26, churned: -7, rewarded: 12700 },
    { month: 'June', value: 52, trend: 45, new: 15, reactivated: 7, active: 30, churned: -5, rewarded: 19100 },
    { month: 'July', value: 48, trend: 48, new: 12, reactivated: 6, active: 30, churned: -8, rewarded: 17400 },
    { month: 'August', value: 55, trend: 50, new: 17, reactivated: 8, active: 30, churned: -4, rewarded: 21300 },
    { month: 'September', value: 50, trend: 52, new: 14, reactivated: 7, active: 29, churned: -6, rewarded: 18600 },
    { month: 'October', value: 58, trend: 54, new: 19, reactivated: 9, active: 30, churned: -5, rewarded: 23800 },
    { month: 'November', value: 52, trend: 56, new: 15, reactivated: 7, active: 30, churned: -7, rewarded: 19500 },
    { month: 'December', value: 48, trend: 52, new: 12, reactivated: 6, active: 30, churned: -8, rewarded: 17200 },
  ];

  // Contributors by country/region
  const contributorsByRegion = [
    { name: 'United Kingdom', value: 625, percentage: 45 },
    { name: 'Germany', value: 720, percentage: 52 },
    { name: 'Canada', value: 580, percentage: 42 },
    { name: 'India', value: 560, percentage: 40 },
    { name: 'Brazil', value: 490, percentage: 35 },
    { name: 'Netherlands', value: 300, percentage: 22 },
    { name: 'Australia', value: 430, percentage: 31 },
    { name: 'Spain', value: 280, percentage: 20 },
    { name: 'Italy', value: 220, percentage: 16 },
    { name: 'Poland', value: 280, percentage: 20 },
    { name: 'Sweden', value: 210, percentage: 15 },
    { name: 'Japan', value: 240, percentage: 17 },
    { name: 'China', value: 220, percentage: 16 },
  ];

  const toggleProjectFilter = (filter: keyof typeof projectFilters) => {
    setProjectFilters(prev => ({ ...prev, [filter]: !prev[filter] }));
  };

  const toggleContributorFilter = (filter: keyof typeof contributorFilters) => {
    setContributorFilters(prev => ({ ...prev, [filter]: !prev[filter] }));
  };

  const CustomTooltip = ({ active, payload }: any) => {
    if (active && payload && payload.length) {
      const data = payload[0].payload;
      return (
        <div className="backdrop-blur-[30px] bg-[#1a1410]/95 border-2 border-white/20 rounded-[12px] px-5 py-4 min-w-[200px]">
          <p className="text-[13px] font-bold text-white mb-3">{data.month} 2025</p>
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <div className="w-3 h-3 rounded-full bg-[#c9983a]" />
                <span className="text-[12px] text-white/80">New</span>
              </div>
              <span className="text-[13px] font-bold text-[#c9983a]">{data.new}</span>
            </div>
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <div className="w-3 h-3 rounded-full bg-[#d4af37]" />
                <span className="text-[12px] text-white/80">Reactivated</span>
              </div>
              <span className="text-[13px] font-bold text-[#d4af37]">{data.reactivated}</span>
            </div>
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <div className="w-3 h-3 rounded-full bg-[#c9983a]/70" />
                <span className="text-[12px] text-white/80">Active</span>
              </div>
              <span className="text-[13px] font-bold text-[#c9983a]/90">{data.active}</span>
            </div>
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <div className="w-3 h-3 rounded-full bg-[#ff6b6b]" />
                <span className="text-[12px] text-white/80">Churned</span>
              </div>
              <span className="text-[13px] font-bold text-[#ff6b6b]">{data.churned}</span>
            </div>
            <div className="h-px bg-white/10 my-2" />
            <div className="flex items-center justify-between pt-1">
              <div className="flex items-center gap-2">
                <div className="w-3 h-3 rounded-full bg-gradient-to-r from-[#c9983a] to-[#d4af37]" />
                <span className="text-[12px] text-white/80">Rewarded</span>
              </div>
              <span className="text-[13px] font-bold text-white">{data.rewarded.toLocaleString()} USD</span>
            </div>
          </div>
        </div>
      );
    }
    return null;
  };

  return (
    <div className="space-y-6">
      {/* Header Tabs */}
      <div className={`backdrop-blur-[40px] rounded-[24px] border p-2 transition-colors ${theme === 'dark'
          ? 'bg-white/[0.12] border-white/20'
          : 'bg-white/[0.12] border-white/20'
        }`}>
        <div className="flex items-center gap-2">
          <button
            onClick={() => setActiveTab('overview')}
            className={`px-6 py-3 rounded-[16px] font-bold text-[14px] transition-all duration-300 ${activeTab === 'overview'
                ? `bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/20 border-2 border-[#c9983a]/50 ${theme === 'dark' ? 'text-[#f5c563]' : 'text-[#2d2820]'
                }`
                : `${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'} hover:bg-white/[0.08]`
              }`}
          >
            Overview
          </button>
          <button
            onClick={() => setActiveTab('projects')}
            className={`px-6 py-3 rounded-[16px] font-bold text-[14px] transition-all duration-300 ${activeTab === 'projects'
                ? `bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/20 border-2 border-[#c9983a]/50 ${theme === 'dark' ? 'text-[#f5c563]' : 'text-[#2d2820]'
                }`
                : `${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'} hover:bg-white/[0.08]`
              }`}
          >
            Projects
          </button>
          <button
            onClick={() => setActiveTab('contributions')}
            className={`px-6 py-3 rounded-[16px] font-bold text-[14px] transition-all duration-300 ${activeTab === 'contributions'
                ? `bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/20 border-2 border-[#c9983a]/50 ${theme === 'dark' ? 'text-[#f5c563]' : 'text-[#2d2820]'
                }`
                : `${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'} hover:bg-white/[0.08]`
              }`}
          >
            Contributions
          </button>
        </div>
      </div>

      {/* Main Grid */}
      <div className="grid grid-cols-2 gap-6">
        {/* Left Column: Project + Contributor stacked */}
        <div className="flex flex-col gap-6">
          {/* Project Activity */}
          <div className="backdrop-blur-[40px] bg-white/[0.12] rounded-[24px] border border-white/20 p-8 flex-1">
            {/* ...All Project Activity JSX... */}
            <div className="h-[140px] mb-6">
              <ResponsiveContainer width="100%" height="100%">
                <ComposedChart data={projectActivityData}>
                  <defs>
                    <linearGradient id="barGradient" x1="0" y1="0" x2="0" y2="1">
                      <stop offset="0%" stopColor="#c9983a" stopOpacity={0.8} />
                      <stop offset="100%" stopColor="#d4af37" stopOpacity={0.4} />
                    </linearGradient>
                  </defs>
                  <CartesianGrid strokeDasharray="3 3" stroke="rgba(122, 107, 90, 0.1)" />
                  <XAxis
                    dataKey="month"
                    stroke="#7a6b5a"
                    tick={{ fill: '#7a6b5a', fontSize: 11, fontWeight: 600 }}
                    angle={-45}
                    textAnchor="end"
                    height={40}
                  />
                  <YAxis stroke="#7a6b5a" tick={{ fill: '#7a6b5a', fontSize: 11, fontWeight: 600 }} />
                  <Tooltip content={<CustomTooltip />} />
                  <Bar dataKey="value" fill="url(#barGradient)" radius={[8, 8, 0, 0]} maxBarSize={40} />
                  <RechartsLine type="monotone" dataKey="trend" stroke="#2d2820" strokeWidth={3} dot={false} />
                </ComposedChart>
              </ResponsiveContainer>
            </div>
          </div>

          {/* Contributor Activity */}
          <div className="backdrop-blur-[40px] bg-white/[0.12] rounded-[24px] border border-white/20 p-8 flex-1">
            {/* ...All Contributor Activity JSX... */}
            <div className="h-[140px] mb-6">
              <ResponsiveContainer width="100%" height="100%">
                <ComposedChart data={contributorActivityData}>
                  <defs>
                    <linearGradient id="contributorBarGradient" x1="0" y1="0" x2="0" y2="1">
                      <stop offset="0%" stopColor="#c9983a" stopOpacity={0.8} />
                      <stop offset="100%" stopColor="#d4af37" stopOpacity={0.4} />
                    </linearGradient>
                  </defs>
                  <CartesianGrid strokeDasharray="3 3" stroke="rgba(122, 107, 90, 0.1)" />
                  <XAxis
                    dataKey="month"
                    stroke="#7a6b5a"
                    tick={{ fill: '#7a6b5a', fontSize: 11, fontWeight: 600 }}
                    angle={-45}
                    textAnchor="end"
                    height={40}
                  />
                  <YAxis stroke="#7a6b5a" tick={{ fill: '#7a6b5a', fontSize: 11, fontWeight: 600 }} />
                  <Tooltip content={<CustomTooltip />} />
                  <Bar dataKey="value" fill="url(#contributorBarGradient)" radius={[8, 8, 0, 0]} maxBarSize={40} />
                  <RechartsLine type="monotone" dataKey="trend" stroke="#2d2820" strokeWidth={3} dot={false} />
                </ComposedChart>
              </ResponsiveContainer>
            </div>
          </div>
        </div>

        {/* Right Column: Contributors Map */}
        <div className="backdrop-blur-[40px] bg-white/[0.12] rounded-[24px] border border-white/20 p-8 flex flex-col">
          <h2 className="text-white font-bold mb-4">Contributors Map</h2>
          <ComposableMap projectionConfig={{ scale: 150 }} width={400} height={300}>
            <ZoomableGroup zoom={mapZoom} center={mapCenter}>
              <Geographies geography={geoUrl}>
                {({ geographies }) =>
                  geographies.map(geo => (
                    <Geography
                      key={geo.rsmKey}
                      geography={geo}
                      fill="#c9983a33"
                      stroke="#c9983a77"
                    />
                  ))
                }
              </Geographies>
              {Object.entries(countryCoordinates).map(([name, coords]) => (
                <Marker key={name} coordinates={coords}>
                  <circle r={5} fill="#c9983a" stroke="#fff" strokeWidth={2} />
                </Marker>
              ))}
            </ZoomableGroup>
          </ComposableMap>
        </div>
      </div>

      {/* Bottom Grid: Info Panel */}
      <div className="grid grid-cols-2 gap-6">
        <div className="backdrop-blur-[40px] bg-white/[0.12] rounded-[24px] border border-white/20 p-8">
          <h2 className="text-white font-bold mb-4">Info Panel</h2>
          {/* Your info content goes here */}
        </div>
      </div>
    </div>
  );
}
