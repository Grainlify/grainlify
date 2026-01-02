import { useState, useEffect } from 'react';
import { X, ExternalLink, User, ChevronDown, Plus, Award, Users, Star, CheckCircle, MessageSquare, Filter, Search, Loader2, AlertCircle } from 'lucide-react';
import { useTheme } from '../../../../shared/contexts/ThemeContext';
import { Issue } from '../../types';
import { EmptyIssueState } from './EmptyIssueState';
import { IssueCard } from '../../../../shared/components/ui/IssueCard';
import { getProjectIssues } from '../../../../shared/api/client';

interface Project {
  id: string;
  github_full_name: string;
  status: string;
}

interface IssuesTabProps {
  onNavigate: (page: string) => void;
  selectedProjects: Project[];
  onRefresh?: () => void;
}

interface IssueFromAPI {
  github_issue_id: number;
  number: number;
  state: string;
  title: string;
  description: string | null;
  author_login: string;
  assignees: any[];
  labels: any[];
  comments_count: number;
  comments: any[];
  url: string;
  updated_at: string | null;
  last_seen_at: string;
}

export function IssuesTab({ onNavigate, selectedProjects, onRefresh }: IssuesTabProps) {
  const { theme } = useTheme();
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedIssue, setSelectedIssue] = useState<Issue | null>(null);
  const [issueDetailTab, setIssueDetailTab] = useState<'applications' | 'discussions'>('applications');
  const [isFilterModalOpen, setIsFilterModalOpen] = useState(false);
  const [selectedFilters, setSelectedFilters] = useState({
    status: [] as string[],
    applicants: [] as string[],
    assignee: [] as string[],
    stale: [] as string[],
    categories: [] as string[],
    languages: [] as string[],
    labels: [] as string[],
  });
  const [labelSearch, setLabelSearch] = useState('');
  const [expandedApplications, setExpandedApplications] = useState<Record<string, boolean>>({});
  const [issues, setIssues] = useState<Array<IssueFromAPI & { projectName: string; projectId: string }>>([]);
  const [isLoadingIssues, setIsLoadingIssues] = useState(true);
  const [issuesError, setIssuesError] = useState<string | null>(null);

  // Fetch issues from selected projects
  useEffect(() => {
    loadIssues();
  }, [selectedProjects]);

  const loadIssues = async () => {
    setIsLoadingIssues(true);
    setIssuesError(null);
    try {
      if (selectedProjects.length === 0) {
        setIssues([]);
        setIsLoadingIssues(false);
        return;
      }

      // Fetch issues from all selected projects in parallel
      const issuePromises = selectedProjects.map(async (project: Project) => {
        try {
          const response = await getProjectIssues(project.id);
          return (response.issues || []).map((issue: IssueFromAPI) => ({
            ...issue,
            projectName: project.github_full_name,
            projectId: project.id,
          }));
        } catch (err) {
          console.error(`Failed to fetch issues for ${project.github_full_name}:`, err);
          return [];
        }
      });

      const allIssues = await Promise.all(issuePromises);
      const flattenedIssues = allIssues.flat();
      
      // Sort by updated_at (most recent first)
      flattenedIssues.sort((a, b) => {
        const dateA = a.updated_at ? new Date(a.updated_at).getTime() : new Date(a.last_seen_at).getTime();
        const dateB = b.updated_at ? new Date(b.updated_at).getTime() : new Date(b.last_seen_at).getTime();
        return dateB - dateA;
      });

      setIssues(flattenedIssues);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to load issues';
      setIssuesError(errorMessage);
      setIssues([]);
    } finally {
      setIsLoadingIssues(false);
    }
  };

  // Refresh issues when selectedProjects change
  // Also refresh when page becomes visible (user switches back to tab)
  // And when repositories are refreshed (new repo added)
  useEffect(() => {
    const handleVisibilityChange = () => {
      if (document.visibilityState === 'visible' && selectedProjects.length > 0) {
        loadIssues();
      }
    };

    const handleRepositoriesRefreshed = () => {
      // Refresh issues when repositories are added/updated
      if (selectedProjects.length > 0) {
        loadIssues();
      }
    };

    document.addEventListener('visibilitychange', handleVisibilityChange);
    window.addEventListener('repositories-refreshed', handleRepositoriesRefreshed);
    
    return () => {
      document.removeEventListener('visibilitychange', handleVisibilityChange);
      window.removeEventListener('repositories-refreshed', handleRepositoriesRefreshed);
    };
  }, [selectedProjects]);

  const handleProfileClick = () => {
    setSelectedIssue(null);
    onNavigate('profile');
  };

  const getApplicationData = (issue: Issue | null) => {
    if (!issue) return null;
    
    return {
      applications: [
        {
          id: '1',
          author: { 
            name: issue.applicant?.name || 'Unknown', 
            avatar: 'https://images.unsplash.com/photo-1535713875002-d1d0cf377fde?w=100&h=100&fit=crop',
          },
          message: issue.applicant?.message || 'Hi, I can handle this task. Kindly assign.',
          timeAgo: issue.applicant?.appliedDate || '3 months ago',
          isAssigned: issue.applicationStatus === 'assigned',
          contributions: issue.applicant?.profileStats?.contributions || 165,
          rewards: issue.applicant?.profileStats?.rewards || 4,
          projectsContributed: issue.applicant?.profileStats?.contributorProjects || 48,
          projectsLead: issue.applicant?.profileStats?.leadProjects || 3,
        },
      ],
      discussions: issue.discussions || [
        {
          id: '1',
          user: issue.applicant?.name || 'Unknown',
          timeAgo: issue.applicant?.appliedDate || '3 months ago',
          content: issue.applicant?.message || 'Hi, I can handle this task. Kindly assign.',
          isAuthor: false,
          appliedForContribution: true,
        },
      ],
    };
  };

  const applicationData = getApplicationData(selectedIssue);
  const isDark = theme === 'dark';
  const appliedFilterCount = selectedFilters.status.length + selectedFilters.applicants.length + selectedFilters.assignee.length + selectedFilters.stale.length + selectedFilters.categories.length + selectedFilters.languages.length + selectedFilters.labels.length;

  return (
    <div className="flex gap-6 h-[calc(100vh-220px)]">
      {/* Left Sidebar - Issues List */}
      <div className="w-[450px] flex-shrink-0 flex flex-col h-full space-y-4">
        {/* Search and Filter Row */}
        <div className="flex items-center gap-3 flex-shrink-0">
          {/* Search Bar */}
          <div className={`flex-1 backdrop-blur-[40px] rounded-[16px] border p-3 transition-colors ${
            isDark
              ? 'bg-white/[0.12] border-white/20'
              : 'bg-white/[0.12] border-white/20'
          }`}>
            <div className="flex items-center gap-2">
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none" className={`flex-shrink-0 ${isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
                <circle cx="7" cy="7" r="5" stroke="currentColor" strokeWidth="1.5"/>
                <path d="m11 11 3 3" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round"/>
              </svg>
              <input
                type="text"
                placeholder="Search"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className={`flex-1 bg-transparent border-none outline-none text-[13px] placeholder:text-[13px] transition-colors ${
                  isDark
                    ? 'text-[#f5f5f5] placeholder-[#d4d4d4]'
                    : 'text-[#2d2820] placeholder-[#7a6b5a]'
                }`}
              />
            </div>
          </div>

          {/* Filter Button with Badge */}
          <button 
            onClick={() => setIsFilterModalOpen(true)}
            className={`relative p-3 rounded-[16px] backdrop-blur-[40px] border hover:bg-white/[0.15] transition-all ${
            isDark
              ? 'bg-white/[0.12] border-white/20'
              : 'bg-white/[0.12] border-white/20'
          }`}>
            <div className="absolute -top-2 -right-2 w-6 h-6 bg-gradient-to-br from-[#e8c571] to-[#c9983a] rounded-full text-[12px] font-bold text-white flex items-center justify-center">
              {appliedFilterCount}
            </div>
            <Filter className={`w-4 h-4 transition-colors ${
              isDark ? 'text-[#2d2820]' : 'text-[#2d2820]'
            }`} />
          </button>
        </div>

        {/* Issues List */}
        <div className="flex-1 overflow-y-auto space-y-3 pr-2 scrollbar-custom">
          {isLoadingIssues ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className={`w-6 h-6 animate-spin ${isDark ? 'text-[#c9983a]' : 'text-[#c9983a]'}`} />
            </div>
          ) : issuesError ? (
            <div className={`flex items-center gap-3 px-4 py-4 mx-4 rounded-[12px] ${
              isDark
                ? 'bg-red-500/10 border border-red-500/30 text-red-400'
                : 'bg-red-100 border border-red-300 text-red-700'
            }`}>
              <AlertCircle className="w-5 h-5 flex-shrink-0" />
              <span className="text-[14px] font-medium">{issuesError}</span>
            </div>
          ) : issues.length === 0 ? (
            <div className={`px-6 py-8 text-center ${
              isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
            }`}>
              <p className="text-[14px] font-medium mb-1">No issues found</p>
              <p className="text-[12px]">
                {selectedProjects.length === 0 
                  ? 'Select repositories to view issues' 
                  : 'No issues in selected repositories'}
              </p>
            </div>
          ) : (
            <>
              {issues
                .filter(issue => {
                  // Search filter
                  const matchesSearch = searchQuery === '' || 
                    issue.title.toLowerCase().includes(searchQuery.toLowerCase()) || 
                    issue.author_login.toLowerCase().includes(searchQuery.toLowerCase());
                  
                  // Status filter
                  const matchesStatus = selectedFilters.status.length === 0 || 
                    selectedFilters.status.includes(issue.state);
                  
                  return matchesSearch && matchesStatus;
                })
                .map((issue) => {
                  // Convert API issue to Issue type for compatibility
                  const issueForCard: Issue = {
                    id: issue.github_issue_id.toString(),
                    title: issue.title,
                    repository: issue.projectName,
                    user: issue.author_login,
                    timeAgo: issue.updated_at 
                      ? new Date(issue.updated_at).toLocaleDateString()
                      : 'Unknown',
                    tags: issue.labels?.map((l: any) => l.name || l) || [],
                    applicants: issue.comments_count || 0,
                    applicant: null,
                    applicationStatus: 'pending',
                    discussions: [],
                  };

                  return (
                    <IssueCard
                      key={`${issue.projectId}-${issue.github_issue_id}`}
                      id={issue.github_issue_id.toString()}
                      number={`#${issue.number}`}
                      title={issue.title}
                      repository={issue.projectName}
                      applicants={issue.comments_count || 0}
                      author={{
                        name: issue.author_login,
                        avatar: `https://github.com/${issue.author_login}.png?size=40`
                      }}
                      timeAgo={issue.updated_at 
                        ? new Date(issue.updated_at).toLocaleDateString()
                        : 'Unknown'}
                      tags={issue.labels?.map((l: any) => l.name || l) || []}
                      isSelected={selectedIssue?.id === issue.github_issue_id.toString()}
                      onClick={() => setSelectedIssue(issueForCard)}
                      showTags={true}
                    />
                  );
                })}

              {/* Issues Count */}
              <div className={`text-center py-2 text-[12px] font-semibold transition-colors ${
                isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
              }`}>
                {issues.length} issue{issues.length !== 1 ? 's' : ''}
              </div>
            </>
          )}
        </div>
      </div>

      {/* Right Content Area - Issue Detail or Placeholder */}
      <div className={`flex-1 backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] relative overflow-y-auto scrollbar-custom transition-colors ${
        isDark
          ? 'bg-[#2d2820]/[0.4] border-white/10'
          : 'bg-white/[0.12] border-white/20'
      }`}>
        {!selectedIssue ? (
          <EmptyIssueState issueCount={issuesData.length} />
        ) : (
          <div className="p-8">
            {/* Header */}
            <div className="flex items-start justify-between mb-6">
              <div className="flex-1">
                <div className="flex items-center gap-3 mb-3">
                  <span className={`text-[24px] font-bold transition-colors ${
                    isDark ? 'text-[#c9983a]' : 'text-[#8b6f3a]'
                  }`}>#{selectedIssue.id}</span>
                  <h1 className={`text-[24px] font-bold transition-colors ${
                    isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                  }`}>
                    {selectedIssue.title}
                  </h1>
                </div>

                <div className="flex items-center gap-3 mb-4">
                  <div className={`flex items-center gap-2 px-3 py-1.5 rounded-[8px] border transition-colors ${
                    isDark 
                      ? 'bg-[#c9983a]/20 border-[#c9983a]/30' 
                      : 'bg-[#8b6f3a]/15 border-[#8b6f3a]/30'
                  }`}>
                    <Star className={`w-4 h-4 transition-colors ${
                      isDark ? 'text-[#c9983a]' : 'text-[#8b6f3a]'
                    }`} />
                    <span className={`text-[12px] font-bold transition-colors ${
                      isDark ? 'text-[#c9983a]' : 'text-[#8b6f3a]'
                    }`}>{selectedIssue.user}</span>
                  </div>
                  <span className={`text-[13px] transition-colors ${isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
                    opened {selectedIssue.timeAgo}
                  </span>
                  <a
                    href="#"
                    className={`flex items-center gap-1 text-[13px] font-semibold hover:underline transition-colors ${
                      isDark ? 'text-[#c9983a]' : 'text-[#8b6f3a]'
                    }`}
                  >
                    View on GitHub
                    <ExternalLink className="w-3 h-3" />
                  </a>
                </div>

                <div className="flex flex-wrap gap-2">
                  {selectedIssue.tags?.map((tag: string, idx: number) => (
                    <span
                      key={idx}
                      className={`px-3 py-1.5 rounded-[8px] text-[12px] font-bold backdrop-blur-[20px] border border-white/25 transition-colors ${
                        isDark ? 'bg-white/[0.08] text-[#d4d4d4]' : 'bg-white/[0.08] text-[#4a3f2f]'
                      }`}
                    >
                      {tag}
                    </span>
                  ))}
                </div>
              </div>

              <button
                onClick={() => setSelectedIssue(null)}
                className={`p-2 rounded-[10px] backdrop-blur-[20px] border border-white/25 hover:bg-white/[0.2] transition-all ${
                  isDark ? 'bg-white/[0.08] text-[#f5f5f5]' : 'bg-white/[0.08] text-[#2d2820]'
                }`}
              >
                <X className="w-5 h-5" />
              </button>
            </div>

            {/* Tabs */}
            <div className="flex items-center gap-2 mb-6 border-b border-white/20 pb-4">
              <button
                onClick={() => setIssueDetailTab('applications')}
                className={`px-4 py-2 rounded-t-[10px] text-[14px] font-semibold transition-all ${
                  issueDetailTab === 'applications'
                    ? 'bg-[#c9983a] text-white'
                    : isDark
                      ? 'text-[#d4d4d4] hover:bg-white/[0.05]'
                      : 'text-[#7a6b5a] hover:bg-white/[0.05]'
                }`}
              >
                Applications {selectedIssue.applicants > 0 && `(${selectedIssue.applicants})`}
              </button>
              <button
                onClick={() => setIssueDetailTab('discussions')}
                className={`px-4 py-2 rounded-t-[10px] text-[14px] font-semibold transition-all ${
                  issueDetailTab === 'discussions'
                    ? 'bg-[#c9983a] text-white'
                    : isDark
                      ? 'text-[#d4d4d4] hover:bg-white/[0.05]'
                      : 'text-[#7a6b5a] hover:bg-white/[0.05]'
                }`}
              >
                Discussions
              </button>
            </div>

            {/* Content */}
            {issueDetailTab === 'applications' && (
              <>
                {selectedIssue.applicationStatus === 'none' && (
                  <div className="text-center py-16">
                    <div className="relative mx-auto mb-6 w-20 h-20">
                      <div className="absolute inset-0 rounded-full bg-gradient-to-br from-[#c9983a]/20 to-[#d4af37]/10 blur-xl" />
                      <div className="relative w-full h-full rounded-full bg-gradient-to-br from-[#c9983a]/15 to-[#d4af37]/10 border-2 border-[#c9983a]/30 flex items-center justify-center backdrop-blur-[20px]">
                        <div className="relative">
                          <User className="w-8 h-8 text-[#c9983a]/60" strokeWidth={1.5} />
                          <Plus className="w-4 h-4 text-[#c9983a] absolute -top-1 -right-1" strokeWidth={3} />
                        </div>
                      </div>
                    </div>
                    <h3 className={`text-[18px] font-bold mb-2 transition-colors ${
                      isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
                    }`}>No applications yet</h3>
                    <p className={`text-[14px] max-w-sm mx-auto leading-relaxed transition-colors ${
                      isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
                    }`}>
                      This issue is open and waiting for contributors to apply.
                      Applications will appear here once submitted.
                    </p>
                  </div>
                )}

                {(selectedIssue.applicationStatus === 'assigned' || selectedIssue.applicationStatus === 'pending') && applicationData && (
                  <div className="space-y-4">
                    {applicationData.applications.map((application) => {
                      const isExpanded = expandedApplications[application.id] || false;
                      
                      return (
                      <div
                        key={application.id}
                        className={`backdrop-blur-[25px] rounded-[16px] border p-6 transition-colors ${
                          isDark ? 'bg-white/[0.15] border-white/25' : 'bg-white/[0.15] border-white/25'
                        }`}
                      >
                        {/* User Header - Always Visible */}
                        <div className="flex items-center justify-between">
                          <button
                            onClick={handleProfileClick}
                            className="flex items-center gap-3 hover:bg-white/10 -m-2 p-2 rounded-[12px] transition-all group/user"
                          >
                            <div className="w-12 h-12 rounded-full bg-gradient-to-br from-[#c9983a] to-[#d4af37] flex items-center justify-center shadow-[0_4px_12px_rgba(201,152,58,0.3)]">
                              <span className="text-[16px] font-bold text-white">
                                {application.author.name.substring(0, 2).toUpperCase()}
                              </span>
                            </div>
                            <div className="text-left">
                              <h4 className={`text-[15px] font-bold transition-colors ${
                                isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
                              } group-hover/user:text-[#c9983a]`}>
                                {application.author.name}
                              </h4>
                              <p className={`text-[12px] transition-colors ${
                                isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
                              }`}>Applied - {application.timeAgo}</p>
                            </div>
                            <ExternalLink className="w-4 h-4 text-[#7a6b5a] ml-auto opacity-0 group-hover/user:opacity-100 transition-opacity" />
                          </button>
                          
                          {/* Dropdown Button */}
                          <button
                            onClick={() => setExpandedApplications(prev => ({
                              ...prev,
                              [application.id]: !prev[application.id]
                            }))}
                            className={`p-2 rounded-[8px] hover:bg-white/10 transition-all ${
                              isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
                            }`}
                          >
                            <ChevronDown className={`w-5 h-5 transition-transform duration-300 ${
                              isExpanded ? 'rotate-180' : ''
                            }`} />
                          </button>
                        </div>

                        {/* Expanded Content */}
                        {isExpanded && (
                          <div className="mt-5 space-y-5">
                            {/* Profile Stats Grid */}
                            <div className="grid grid-cols-2 gap-3">
                              <div className={`backdrop-blur-[20px] rounded-[12px] border border-[#c9983a]/20 p-3 transition-colors ${
                                isDark ? 'bg-white/[0.12]' : 'bg-white/[0.12]'
                              }`}>
                                <div className="flex items-center gap-2 mb-1">
                                  <Award className={`w-4 h-4 transition-colors ${
                                    isDark ? 'text-[#c9983a]' : 'text-[#c9983a]'
                                  }`} />
                                  <span className={`text-[20px] font-bold transition-colors ${
                                    isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
                                  }`}>{application.contributions}</span>
                                </div>
                                <p className={`text-[11px] font-semibold uppercase tracking-wide transition-colors ${
                                  isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
                                }`}>Contributions</p>
                              </div>
                              <div className={`backdrop-blur-[20px] rounded-[12px] border border-[#c9983a]/20 p-3 transition-colors ${
                                isDark ? 'bg-white/[0.12]' : 'bg-white/[0.12]'
                              }`}>
                                <div className="flex items-center gap-2 mb-1">
                                  <Award className={`w-4 h-4 transition-colors ${
                                    isDark ? 'text-[#c9983a]' : 'text-[#c9983a]'
                                  }`} />
                                  <span className={`text-[20px] font-bold transition-colors ${
                                    isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
                                  }`}>{application.rewards}</span>
                                </div>
                                <p className={`text-[11px] font-semibold uppercase tracking-wide transition-colors ${
                                  isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
                                }`}>Rewards</p>
                              </div>
                            </div>

                            {/* Additional Profile Info */}
                            <div className="space-y-2">
                              <div className="flex items-center gap-2">
                                <Users className={`w-4 h-4 transition-colors ${
                                  isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
                                }`} />
                                <span className={`text-[13px] transition-colors ${
                                  isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
                                }`}>
                                  Contributor on{' '}
                                  <span className={`font-bold transition-colors ${
                                    isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
                                  }`}>{application.projectsContributed}</span>
                                  {' '}projects
                                </span>
                              </div>
                              <div className="flex items-center gap-2">
                                <Star className={`w-4 h-4 transition-colors ${
                                  isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
                                }`} />
                                <span className={`text-[13px] transition-colors ${
                                  isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
                                }`}>
                                  Lead{' '}
                                  <span className={`font-bold transition-colors ${
                                    isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
                                  }`}>{application.projectsLead}</span>
                                  {' '}projects
                                </span>
                              </div>
                            </div>

                            {/* Message */}
                            <div className={`p-4 rounded-[12px] border transition-colors ${
                              isDark 
                                ? 'bg-white/20 border-white/30' 
                                : 'bg-white/20 border-white/30'
                            }`}>
                              <p className={`text-[13px] leading-relaxed transition-colors ${
                                isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
                              }`}>
                                {application.message}
                              </p>
                            </div>

                            {/* Status & Action Buttons */}
                            <div className="flex items-center justify-between">
                              {selectedIssue.applicationStatus === 'assigned' ? (
                                <>
                                  <div className="flex items-center gap-2">
                                    <div className="w-5 h-5 rounded-full bg-gradient-to-br from-[#c9983a] to-[#d4af37] flex items-center justify-center">
                                      <CheckCircle className="w-3 h-3 text-white" strokeWidth={3} />
                                    </div>
                                    <span className={`text-[13px] font-bold transition-colors ${
                                      isDark ? 'text-[#c9983a]' : 'text-[#c9983a]'
                                    }`}>Assigned</span>
                                  </div>
                                  <button className={`px-4 py-2 rounded-[8px] border text-[13px] font-semibold transition-all ${
                                    isDark
                                      ? 'bg-white/30 hover:bg-white/50 border-white/40 hover:border-[#c9983a]/40 text-[#e8dfd0] hover:text-[#c9983a]'
                                      : 'bg-white/30 hover:bg-white/50 border-white/40 hover:border-[#c9983a]/40 text-[#2d2820] hover:text-[#c9983a]'
                                  }`}>
                                    Unassign
                                  </button>
                                </>
                              ) : (
                                <>
                                  <button className={`flex-1 px-4 py-2 rounded-[8px] border text-[13px] font-semibold transition-all mr-2 ${
                                    isDark
                                      ? 'bg-white/30 hover:bg-white/50 border-white/40 hover:border-[#c9983a]/40 text-[#e8dfd0] hover:text-[#c9983a]'
                                      : 'bg-white/30 hover:bg-white/50 border-white/40 hover:border-[#c9983a]/40 text-[#2d2820] hover:text-[#c9983a]'
                                  }`}>
                                    Reject
                                  </button>
                                  <button className="flex-1 px-4 py-2 rounded-[8px] bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/25 border border-[#c9983a]/40 text-[13px] font-semibold text-[#2d2820] hover:from-[#c9983a]/40 hover:to-[#d4af37]/35 hover:shadow-[0_4px_16px_rgba(201,152,58,0.3)] transition-all">
                                    Assign
                                  </button>
                                </>
                              )}
                            </div>
                          </div>
                        )}
                      </div>
                      );
                    })}
                  </div>
                )}
              </>
            )}

            {issueDetailTab === 'discussions' && (
              <div className="space-y-4">
                {applicationData && applicationData.discussions.length > 0 ? (
                  applicationData.discussions.map((discussion) => (
                    <div
                      key={discussion.id}
                      className={`backdrop-blur-[25px] rounded-[16px] border p-5 transition-colors ${
                        isDark
                          ? 'bg-white/[0.08] border-white/10'
                          : 'bg-white/[0.15] border-white/25'
                      }`}
                    >
                      <div className="flex items-center gap-3 mb-3">
                        <div className="w-8 h-8 rounded-full bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/20 border border-[#c9983a]/40 flex items-center justify-center">
                          <span className="text-[11px] font-bold text-[#c9983a]">
                            {discussion.user.substring(0, 2).toUpperCase()}
                          </span>
                        </div>
                        <div>
                          <div className="flex items-center gap-2">
                            <span className={`text-[14px] font-bold transition-colors ${
                              isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
                            }`}>{discussion.user}</span>
                            {discussion.isAuthor && (
                              <span className="px-2 py-0.5 rounded-[4px] bg-[#c9983a]/20 border border-[#c9983a]/30 text-[10px] font-bold text-[#c9983a]">
                                AUTHOR
                              </span>
                            )}
                          </div>
                          <span className={`text-[12px] transition-colors ${
                            isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
                          }`}>{discussion.timeAgo}</span>
                        </div>
                      </div>
                      
                      {discussion.appliedForContribution && (
                        <div className="mb-3 px-3 py-2 rounded-[8px] bg-[#c9983a]/10 border border-[#c9983a]/20">
                          <span className="text-[12px] font-semibold text-[#c9983a]">
                            Applied for this contribution
                          </span>
                        </div>
                      )}

                      <div className={`text-[14px] leading-relaxed whitespace-pre-wrap transition-colors ${
                        isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
                      }`}>
                        {discussion.content}
                      </div>
                    </div>
                  ))
                ) : (
                  <div className={`p-8 rounded-[16px] backdrop-blur-[25px] border text-center min-h-[300px] flex flex-col items-center justify-center ${
                    isDark ? 'bg-white/[0.08] border-white/10' : 'bg-white/[0.15] border-white/25'
                  }`}>
                    <MessageSquare className={`w-12 h-12 mx-auto mb-4 transition-colors ${
                      isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                    }`} />
                    <p className={`text-[14px] transition-colors ${
                      isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                    }`}>
                      No discussions yet
                    </p>
                  </div>
                )}
              </div>
            )}
          </div>
        )}
      </div>

      {/* Filter Modal */}
      {isFilterModalOpen && (
        <>
          <div 
            className="fixed inset-0 z-40"
            onClick={() => setIsFilterModalOpen(false)}
          />
          
          <div 
            className={`fixed top-[140px] right-[350px] z-50 w-[350px] max-h-[calc(100vh-160px)] flex flex-col rounded-[20px] border-2 shadow-[0_20px_60px_rgba(0,0,0,0.3)] transition-colors ${
              isDark
                ? 'bg-[#3a3228] border-white/30'
                : 'bg-[#d4c5b0] border-white/40'
            }`}
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center justify-between p-6 pb-4 flex-shrink-0 border-b border-white/10">
              <div className="flex items-center gap-3">
                <div className={`w-10 h-10 rounded-[12px] flex items-center justify-center shadow-lg border-2 ${
                  isDark
                    ? 'bg-gradient-to-br from-[#e8c571]/30 via-[#d4af37]/25 to-[#c9983a]/20 border-[#e8c571]/50'
                    : 'bg-gradient-to-br from-[#c9983a]/30 via-[#d4af37]/25 to-[#c9983a]/20 border-[#c9983a]/50'
                }`}>
                  <Filter className="w-5 h-5 text-white" />
                </div>
                <h2 className={`text-[18px] font-bold transition-colors ${
                  isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
                }`}>All Filters</h2>
              </div>
              <button
                onClick={() => setIsFilterModalOpen(false)}
                className={`p-2 rounded-[10px] transition-all hover:scale-110 ${
                  isDark
                    ? 'hover:bg-white/[0.1] text-[#e8c571] hover:text-[#f5d98a]'
                    : 'hover:bg-black/[0.05] text-[#8b6f3a] hover:text-[#c9983a]'
                }`}
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <div className="flex-1 overflow-y-auto p-6 scrollbar-custom space-y-4">
              {/* Status & Applicants - Two Column */}
              <div className="grid grid-cols-2 gap-3">
                {/* Status */}
                <div>
                  <h3 className={`text-[12px] font-semibold mb-2 transition-colors ${
                    isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
                  }`}>Status</h3>
                  <div className="flex gap-2">
                    {['Open', 'Close'].map((status) => (
                      <button
                        key={status}
                        onClick={() => {
                          const lowerStatus = status.toLowerCase();
                          if (selectedFilters.status.includes(lowerStatus)) {
                            setSelectedFilters(prev => ({
                              ...prev,
                              status: prev.status.filter(s => s !== lowerStatus)
                            }));
                          } else {
                            setSelectedFilters(prev => ({
                              ...prev,
                              status: [...prev.status, lowerStatus]
                            }));
                          }
                        }}
                        className={`flex-1 px-2 py-1.5 rounded-[8px] text-[12px] font-semibold transition-all border ${
                          selectedFilters.status.includes(status.toLowerCase())
                            ? 'bg-[#c9983a]/20 border-[#c9983a] text-[#c9983a]'
                            : isDark
                              ? 'bg-white/[0.08] border-white/15 text-[#e8dfd0] hover:bg-white/[0.12]'
                              : 'bg-white/[0.15] border-white/25 text-[#7a6b5a] hover:bg-white/[0.2]'
                        }`}
                      >
                        {status}
                      </button>
                    ))}
                  </div>
                </div>

                {/* Applicants */}
                <div>
                  <h3 className={`text-[12px] font-semibold mb-2 transition-colors ${
                    isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
                  }`}>Applicants</h3>
                  <div className="flex gap-2">
                    {['Yes', 'No'].map((applicant) => (
                      <button
                        key={applicant}
                        onClick={() => {
                          const lowerApplicant = applicant.toLowerCase();
                          if (selectedFilters.applicants.includes(lowerApplicant)) {
                            setSelectedFilters(prev => ({
                              ...prev,
                              applicants: prev.applicants.filter(a => a !== lowerApplicant)
                            }));
                          } else {
                            setSelectedFilters(prev => ({
                              ...prev,
                              applicants: [...prev.applicants, lowerApplicant]
                            }));
                          }
                        }}
                        className={`flex-1 px-2 py-1.5 rounded-[8px] text-[12px] font-semibold transition-all border ${
                          selectedFilters.applicants.includes(applicant.toLowerCase())
                            ? 'bg-[#c9983a]/20 border-[#c9983a] text-[#c9983a]'
                            : isDark
                              ? 'bg-white/[0.08] border-white/15 text-[#e8dfd0] hover:bg-white/[0.12]'
                              : 'bg-white/[0.15] border-white/25 text-[#7a6b5a] hover:bg-white/[0.2]'
                        }`}
                      >
                        {applicant}
                      </button>
                    ))}
                  </div>
                </div>
              </div>

              {/* Assignee & Stale - Two Column */}
              <div className="grid grid-cols-2 gap-3">
                {/* Assignee */}
                <div>
                  <h3 className={`text-[12px] font-semibold mb-2 transition-colors ${
                    isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
                  }`}>Assignee</h3>
                  <div className="flex gap-2">
                    {['Yes', 'No'].map((assignee) => (
                      <button
                        key={assignee}
                        onClick={() => {
                          const lowerAssignee = assignee.toLowerCase();
                          if (selectedFilters.assignee.includes(lowerAssignee)) {
                            setSelectedFilters(prev => ({
                              ...prev,
                              assignee: prev.assignee.filter(a => a !== lowerAssignee)
                            }));
                          } else {
                            setSelectedFilters(prev => ({
                              ...prev,
                              assignee: [...prev.assignee, lowerAssignee]
                            }));
                          }
                        }}
                        className={`flex-1 px-2 py-1.5 rounded-[8px] text-[12px] font-semibold transition-all border ${
                          selectedFilters.assignee.includes(assignee.toLowerCase())
                            ? 'bg-[#c9983a]/20 border-[#c9983a] text-[#c9983a]'
                            : isDark
                              ? 'bg-white/[0.08] border-white/15 text-[#e8dfd0] hover:bg-white/[0.12]'
                              : 'bg-white/[0.15] border-white/25 text-[#7a6b5a] hover:bg-white/[0.2]'
                        }`}
                      >
                        {assignee}
                      </button>
                    ))}
                  </div>
                </div>

                {/* Stale */}
                <div>
                  <h3 className={`text-[12px] font-semibold mb-2 transition-colors ${
                    isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
                  }`}>Stale</h3>
                  <div className="flex gap-2">
                    {['Yes', 'No'].map((stale) => (
                      <button
                        key={stale}
                        onClick={() => {
                          const lowerStale = stale.toLowerCase();
                          if (selectedFilters.stale.includes(lowerStale)) {
                            setSelectedFilters(prev => ({
                              ...prev,
                              stale: prev.stale.filter(s => s !== lowerStale)
                            }));
                          } else {
                            setSelectedFilters(prev => ({
                              ...prev,
                              stale: [...prev.stale, lowerStale]
                            }));
                          }
                        }}
                        className={`flex-1 px-2 py-1.5 rounded-[8px] text-[12px] font-semibold transition-all border ${
                          selectedFilters.stale.includes(stale.toLowerCase())
                            ? 'bg-[#c9983a]/20 border-[#c9983a] text-[#c9983a]'
                            : isDark
                              ? 'bg-white/[0.08] border-white/15 text-[#e8dfd0] hover:bg-white/[0.12]'
                              : 'bg-white/[0.15] border-white/25 text-[#7a6b5a] hover:bg-white/[0.2]'
                        }`}
                      >
                        {stale}
                      </button>
                    ))}
                  </div>
                </div>
              </div>

              {/* Categories */}
              <div>
                <h3 className={`text-[12px] font-semibold mb-2 transition-colors ${
                  isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
                }`}>Categories</h3>
                <div className="flex flex-wrap gap-2">
                  {['Blockchain & Cryptocurrencies', 'Cryptography', 'Stellar', 'Web Development'].map((category) => (
                    <button
                      key={category}
                      onClick={() => {
                        const lowerCategory = category.toLowerCase();
                        if (selectedFilters.categories.includes(lowerCategory)) {
                          setSelectedFilters(prev => ({
                            ...prev,
                            categories: prev.categories.filter(c => c !== lowerCategory)
                          }));
                        } else {
                          setSelectedFilters(prev => ({
                            ...prev,
                            categories: [...prev.categories, lowerCategory]
                          }));
                        }
                      }}
                      className={`px-2.5 py-1.5 rounded-[8px] text-[11px] font-semibold transition-all border ${
                        selectedFilters.categories.includes(category.toLowerCase())
                          ? 'bg-[#c9983a]/20 border-[#c9983a] text-[#c9983a]'
                          : isDark
                            ? 'bg-white/[0.08] border-white/15 text-[#e8dfd0] hover:bg-white/[0.12]'
                            : 'bg-white/[0.15] border-white/25 text-[#7a6b5a] hover:bg-white/[0.2]'
                      }`}
                    >
                      {category}
                    </button>
                  ))}
                </div>
              </div>

              {/* Languages */}
              <div>
                <h3 className={`text-[12px] font-semibold mb-2 transition-colors ${
                  isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
                }`}>Languages</h3>
                <div className="flex flex-wrap gap-2">
                  {['JavaScript', 'Makefile', 'Rust', 'Shell', 'TypeScript'].map((language) => (
                    <button
                      key={language}
                      onClick={() => {
                        const lowerLanguage = language.toLowerCase();
                        if (selectedFilters.languages.includes(lowerLanguage)) {
                          setSelectedFilters(prev => ({
                            ...prev,
                            languages: prev.languages.filter(l => l !== lowerLanguage)
                          }));
                        } else {
                          setSelectedFilters(prev => ({
                            ...prev,
                            languages: [...prev.languages, lowerLanguage]
                          }));
                        }
                      }}
                      className={`px-2.5 py-1.5 rounded-[8px] text-[11px] font-semibold transition-all border ${
                        selectedFilters.languages.includes(language.toLowerCase())
                          ? 'bg-[#c9983a]/20 border-[#c9983a] text-[#c9983a]'
                          : isDark
                            ? 'bg-white/[0.08] border-white/15 text-[#e8dfd0] hover:bg-white/[0.12]'
                            : 'bg-white/[0.15] border-white/25 text-[#7a6b5a] hover:bg-white/[0.2]'
                      }`}
                    >
                      {language}
                    </button>
                  ))}
                </div>
              </div>

              {/* Labels */}
              <div>
                <h3 className={`text-[12px] font-semibold mb-2 transition-colors ${
                  isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
                }`}>Labels</h3>
                
                {/* Search Bar */}
                <div className={`mb-2.5 px-3 py-2 rounded-[8px] border transition-colors ${
                  isDark
                    ? 'bg-white/[0.08] border-white/15'
                    : 'bg-white/[0.15] border-white/25'
                }`}>
                  <div className="flex items-center gap-2">
                    <Search className={`w-3.5 h-3.5 flex-shrink-0 ${isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'}`} />
                    <input
                      type="text"
                      placeholder="Search"
                      value={labelSearch}
                      onChange={(e) => setLabelSearch(e.target.value)}
                      className={`flex-1 bg-transparent border-none outline-none text-[12px] placeholder:text-[12px] transition-colors ${
                        isDark
                          ? 'text-[#e8dfd0] placeholder-[#b8a898]/60'
                          : 'text-[#2d2820] placeholder-[#7a6b5a]/60'
                      }`}
                    />
                  </div>
                </div>

                {/* Label Pills */}
                <div className="flex flex-wrap gap-2">
                  {['bug', 'Difficulty: easy', 'documentation'].filter(label => label.toLowerCase().includes(labelSearch.toLowerCase())).map((label) => (
                    <button
                      key={label}
                      onClick={() => {
                        const lowerLabel = label.toLowerCase();
                        if (selectedFilters.labels.includes(lowerLabel)) {
                          setSelectedFilters(prev => ({
                            ...prev,
                            labels: prev.labels.filter(l => l !== lowerLabel)
                          }));
                        } else {
                          setSelectedFilters(prev => ({
                            ...prev,
                            labels: [...prev.labels, lowerLabel]
                          }));
                        }
                      }}
                      className={`px-2.5 py-1.5 rounded-[8px] text-[11px] font-semibold transition-all border ${
                        selectedFilters.labels.includes(label.toLowerCase())
                          ? 'bg-[#c9983a]/20 border-[#c9983a] text-[#c9983a]'
                          : isDark
                            ? 'bg-white/[0.08] border-white/15 text-[#e8dfd0] hover:bg-white/[0.12]'
                            : 'bg-white/[0.15] border-white/25 text-[#7a6b5a] hover:bg-white/[0.2]'
                      }`}
                    >
                      {label}
                    </button>
                  ))}
                </div>
              </div>
            </div>

            <div className="flex items-center justify-between gap-3 p-6 pt-4 flex-shrink-0 border-t border-white/10">
              <button
                onClick={() => {
                  setSelectedFilters({
                    status: [],
                    applicants: [],
                    assignee: [],
                    stale: [],
                    categories: [],
                    languages: [],
                    labels: [],
                  });
                  setLabelSearch('');
                }}
                className={`px-4 py-2 rounded-[10px] text-[12px] font-semibold transition-all hover:scale-[1.02] ${
                  isDark
                    ? 'text-[#e8dfd0] hover:bg-white/[0.05]'
                    : 'text-[#7a6b5a] hover:bg-white/[0.1]'
                }`}
              >
                Clear filters
              </button>
              <button
                onClick={() => setIsFilterModalOpen(false)}
                className="px-5 py-2 rounded-[10px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white font-semibold text-[12px] shadow-[0_6px_20px_rgba(162,121,44,0.35)] hover:shadow-[0_8px_24px_rgba(162,121,44,0.5)] transition-all border border-white/10 hover:scale-[1.02]"
              >
                Apply
              </button>
            </div>
          </div>
        </>
      )}
    </div>
  );
}