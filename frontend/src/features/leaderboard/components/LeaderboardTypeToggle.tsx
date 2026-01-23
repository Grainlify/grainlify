import { Users, FolderGit2 } from 'lucide-react';
import { LeaderboardType } from '../types';
import { useIsMobile } from '../../../app/components/ui/use-mobile';

interface LeaderboardTypeToggleProps {
  leaderboardType: LeaderboardType;
  onToggle: (type: LeaderboardType) => void;
  isLoaded: boolean;
}

export function LeaderboardTypeToggle({ leaderboardType, onToggle, isLoaded }: LeaderboardTypeToggleProps) {
  const isMobile = useIsMobile();
  
  return (
    <div className={`sticky top-3 md:top-6 z-[200] flex justify-center transition-all duration-1000 px-4 ${
      isLoaded ? 'opacity-100 translate-y-0' : 'opacity-0 -translate-y-4'
    }`}>
      <div className="backdrop-blur-[40px] bg-gradient-to-br from-white/[0.25] to-white/[0.15] rounded-[16px] md:rounded-[20px] border-2 border-white/30 shadow-[0_12px_48px_rgba(0,0,0,0.15)] p-1.5 md:p-2 flex gap-1.5 md:gap-2 w-full max-w-[400px] md:max-w-none">
        <button
          onClick={() => onToggle('contributors')}
          className={`flex items-center justify-center gap-1.5 md:gap-2 ${isMobile ? 'px-3 py-2.5 flex-1' : 'px-6 py-3'} rounded-[12px] md:rounded-[14px] font-bold text-[13px] md:text-[15px] transition-all duration-300 min-h-[44px] active:scale-95 ${
            leaderboardType === 'contributors'
              ? 'bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white shadow-[0_6px_20px_rgba(201,152,58,0.4)] scale-105 border-2 border-white/20'
              : 'text-[#6b5d4d] hover:bg-white/[0.15] hover:scale-105'
          }`}
        >
          <Users className="w-4 h-4 md:w-5 md:h-5" />
          <span className={isMobile ? 'text-xs' : ''}>Contributors</span>
        </button>
        <button
          onClick={() => onToggle('projects')}
          className={`flex items-center justify-center gap-1.5 md:gap-2 ${isMobile ? 'px-3 py-2.5 flex-1' : 'px-6 py-3'} rounded-[12px] md:rounded-[14px] font-bold text-[13px] md:text-[15px] transition-all duration-300 min-h-[44px] active:scale-95 ${
            leaderboardType === 'projects'
              ? 'bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white shadow-[0_6px_20px_rgba(201,152,58,0.4)] scale-105 border-2 border-white/20'
              : 'text-[#6b5d4d] hover:bg-white/[0.15] hover:scale-105'
          }`}
        >
          <FolderGit2 className="w-4 h-4 md:w-5 md:h-5" />
          <span className={isMobile ? 'text-xs' : ''}>Projects</span>
        </button>
      </div>
    </div>
  );
}
