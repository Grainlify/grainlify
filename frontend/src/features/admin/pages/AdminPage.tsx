import { useState, useEffect, useCallback } from 'react';
import { useTheme } from '../../../shared/contexts/ThemeContext';
import { Shield, Globe, Plus, Sparkles, Trash2, ExternalLink, Calendar, Package } from 'lucide-react';
import { Modal, ModalFooter, ModalButton, ModalInput, ModalSelect } from '../../../shared/components/ui/Modal';
import {
  createEcosystem,
  getAdminEcosystems,
  deleteEcosystem,
  createOpenSourceWeekEvent,
  getAdminOpenSourceWeekEvents,
  deleteOpenSourceWeekEvent,
  getAdminProjects,
  deleteAdminProject,
  createProject,
  verifyProject,
  syncProject
} from '../../../shared/api/client';
import { ProjectCard, Project } from '../../dashboard/components/ProjectCard';

// Helper functions (consistent with BrowsePage)
const formatNumber = (num: number): string => {
  if (num >= 1000000) return `${(num / 1000000).toFixed(1)}M`;
  if (num >= 1000) return `${(num / 1000).toFixed(1)}K`;
  return num.toString();
};

const getProjectIcon = (githubFullName: string): string => {
  const [owner] = githubFullName.split('/');
  return `https://github.com/${owner}.png?size=40`;
};

const getProjectColor = (name: string): string => {
  const colors = [
    'from-blue-500 to-cyan-500',
    'from-purple-500 to-pink-500',
    'from-green-500 to-emerald-500',
    'from-red-500 to-pink-500',
    'from-orange-500 to-red-500',
    'from-gray-600 to-gray-800',
    'from-green-600 to-green-800',
    'from-cyan-500 to-blue-600',
  ];
  const hash = name.split('').reduce((acc, char) => acc + char.charCodeAt(0), 0);
  return colors[hash % colors.length];
};

const truncateDescription = (description: string | undefined | null, maxLength: number = 80): string => {
  if (!description || description.trim() === '') return '';
  const firstLine = description.split('\n')[0].trim();
  return firstLine.length > maxLength ? firstLine.substring(0, maxLength).trim() + '...' : firstLine;
};

interface Ecosystem {
  id: string;
  slug: string;
  name: string;
  description: string | null;
  website_url: string | null;
  status: string;
  project_count: number;
  user_count: number;
  created_at: string;
  updated_at: string;
}

export function AdminPage() {
  const { theme } = useTheme();
  const [showAddModal, setShowAddModal] = useState(false);
  const [ecosystems, setEcosystems] = useState<Ecosystem[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [deleteConfirm, setDeleteConfirm] = useState<{ id: string; name: string } | null>(null);
  const [activeTab, setActiveTab] = useState<'ecosystems' | 'projects' | 'events'>('ecosystems');
  const [formData, setFormData] = useState({
    name: '',
    description: '',
    status: 'active',
    websiteUrl: ''
  });
  const [errors, setErrors] = useState<Record<string, string>>({});

  const validateName = (name: string) => {
    if (!name.trim()) return 'Ecosystem name is required';
    if (name.length < 2) return 'Ecosystem name must be at least 2 characters';
    if (name.length > 100) return 'Ecosystem name must be less than 100 characters';
    if (!/^[a-zA-Z0-9\s-]+$/.test(name)) return 'Name can only contain letters, numbers, spaces, and hyphens';
    return null;
  };

  const validateDescription = (description: string) => {
    if (!description.trim()) return 'Description is required';
    if (description.length < 10) return 'Description must be at least 10 characters';
    if (description.length > 500) return 'Description must be less than 500 characters';
    return null;
  };

  const validateWebsiteUrl = (url: string) => {
    if (!url.trim()) return 'Website URL is required';
    try {
      new URL(url);
      if (!url.startsWith('http')) return 'URL must start with http:// or https://';
      return null;
    } catch {
      return 'Please enter a valid URL (e.g., https://example.com)';
    }
  };

  const [isSubmitting, setIsSubmitting] = useState(false);

  // Project Management
  const [adminProjects, setAdminProjects] = useState<Project[]>([]);
  const [isAdminProjectsLoading, setIsAdminProjectsLoading] = useState(true);
  const [projectDeleteConfirm, setProjectDeleteConfirm] = useState<{ id: string; name: string } | null>(null);
  const [isDeletingProject, setIsDeletingProject] = useState(false);
  const [showAddProjectModal, setShowAddProjectModal] = useState(false);
  const [projectFormData, setProjectFormData] = useState({
    github_full_name: '',
    ecosystem_name: '',
    language: '',
    tags: '',
    category: 'other'
  });

  // Automatically select first ecosystem when modal opens
  useEffect(() => {
    if (showAddProjectModal && ecosystems.length > 0 && !projectFormData.ecosystem_name) {
      setProjectFormData(prev => ({ ...prev, ecosystem_name: ecosystems[0].name }));
    }
  }, [showAddProjectModal, ecosystems, projectFormData.ecosystem_name]);

  const fetchAdminProjects = useCallback(async () => {
    try {
      setIsAdminProjectsLoading(true);
      setErrorMessage(null);
      console.log('AdminPage: Fetching admin projects...');
      const response = await getAdminProjects();
      console.log('AdminPage: Admin projects response:', response);

      const mappedProjects: Project[] = (response.projects || []).map((p: any) => ({
        id: p.id,
        name: p.github_full_name.split('/')[1] || p.github_full_name,
        icon: getProjectIcon(p.github_full_name),
        stars: formatNumber(p.stars_count || 0),
        forks: formatNumber(p.forks_count || 0),
        contributors: p.contributors_count || 0,
        openIssues: p.open_issues_count || 0,
        prs: p.open_prs_count || 0,
        description: truncateDescription(p.description) || `${p.language || p.category || 'Project'} repository`,
        tags: Array.isArray(p.tags) ? p.tags : [],
        color: getProjectColor(p.github_full_name.split('/')[1] || p.github_full_name),
      }));

      console.log('AdminPage: Mapped projects:', mappedProjects.length);
      setAdminProjects(mappedProjects);
    } catch (error) {
      console.error('AdminPage: Failed to fetch admin projects:', error);
      const msg = error instanceof Error ? error.message : 'Failed to load projects.';
      setErrorMessage(msg);
      setAdminProjects([]);
    } finally {
      setIsAdminProjectsLoading(false);
    }
  }, []);

  // Open Source Week events
  const [oswEvents, setOswEvents] = useState<Array<{
    id: string;
    title: string;
    description: string | null;
    location: string | null;
    status: string;
    start_at: string;
    end_at: string;
  }>>([]);
  const [isOswLoading, setIsOswLoading] = useState(true);
  const [showAddOswModal, setShowAddOswModal] = useState(false);
  const [oswDeletingId, setOswDeletingId] = useState<string | null>(null);
  const [oswDeleteConfirm, setOswDeleteConfirm] = useState<{ id: string; title: string } | null>(null);
  const [oswForm, setOswForm] = useState({
    title: '',
    description: '',
    location: '',
    status: 'upcoming',
    startDate: '',
    startTime: '00:00',
    endDate: '',
    endTime: '00:00',
  });

  const fetchOswEvents = async () => {
    try {
      setIsOswLoading(true);
      const res = await getAdminOpenSourceWeekEvents();
      setOswEvents(res.events || []);
    } catch (e) {
      setOswEvents([]);
    } finally {
      setIsOswLoading(false);
    }
  };

  const fetchEcosystems = async () => {
    try {
      setIsLoading(true);
      setErrorMessage(null);
      const response = await getAdminEcosystems();
      setEcosystems(response.ecosystems || []);
    } catch (error) {
      console.error('Failed to fetch ecosystems:', error);
      setEcosystems([]);
      setErrorMessage(error instanceof Error ? error.message : 'Failed to load ecosystems.');
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    fetchEcosystems();
    fetchOswEvents();
    fetchAdminProjects();

    const handleEcosystemsUpdated = () => fetchEcosystems();
    const handleProjectsUpdated = () => fetchAdminProjects();

    window.addEventListener('ecosystems-updated', handleEcosystemsUpdated);
    window.addEventListener('projects-updated', handleProjectsUpdated);
    return () => {
      window.removeEventListener('ecosystems-updated', handleEcosystemsUpdated);
      window.removeEventListener('projects-updated', handleProjectsUpdated);
    };
  }, [fetchAdminProjects]);

  const confirmDeleteOsw = (id: string, title: string) => {
    setOswDeleteConfirm({ id, title });
  };

  const handleDeleteOswConfirmed = async () => {
    if (!oswDeleteConfirm) return;
    setOswDeletingId(oswDeleteConfirm.id);
    try {
      await deleteOpenSourceWeekEvent(oswDeleteConfirm.id);
      await fetchOswEvents();
      setOswDeleteConfirm(null);
    } catch (e) {
      setErrorMessage(e instanceof Error ? e.message : 'Failed to delete event.');
    } finally {
      setOswDeletingId(null);
    }
  };

  const confirmDeleteProject = (e: React.MouseEvent, id: string, name: string) => {
    e.stopPropagation();
    setProjectDeleteConfirm({ id, name });
  };

  const handleDeleteProjectConfirmed = async () => {
    if (!projectDeleteConfirm) return;
    setIsDeletingProject(true);
    try {
      await deleteAdminProject(projectDeleteConfirm.id);
      await fetchAdminProjects();
      setProjectDeleteConfirm(null);
    } catch (e) {
      setErrorMessage(e instanceof Error ? e.message : 'Failed to delete project.');
    } finally {
      setIsDeletingProject(false);
    }
  };

  const handleCreateProject = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!projectFormData.github_full_name.includes('/')) {
      setErrorMessage('GitHub Repository must be in the format "owner/repo"');
      return;
    }

    setIsSubmitting(true);
    try {
      setErrorMessage(null);
      await createProject({
        github_full_name: projectFormData.github_full_name,
        ecosystem_name: projectFormData.ecosystem_name,
        language: projectFormData.language || undefined,
        tags: projectFormData.tags ? projectFormData.tags.split(',').map(t => t.trim()).filter(t => t !== '') : [],
        category: projectFormData.category || undefined,
      });
      setShowAddProjectModal(false);
      setProjectFormData({
        github_full_name: '',
        ecosystem_name: ecosystems[0]?.name || '',
        language: '',
        tags: '',
        category: 'other'
      });
      await fetchAdminProjects();
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to create project.';
      setErrorMessage(msg);
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleVerifyProject = async (id: string) => {
    try {
      await verifyProject(id);
      await fetchAdminProjects();
    } catch (e) {
      setErrorMessage(e instanceof Error ? e.message : 'Failed to verify project.');
    }
  };

  const handleCreateOsw = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsSubmitting(true);
    try {
      setErrorMessage(null);
      const start_at = new Date(`${oswForm.startDate}T${oswForm.startTime}:00.000Z`).toISOString();
      const end_at = new Date(`${oswForm.endDate}T${oswForm.endTime}:00.000Z`).toISOString();
      await createOpenSourceWeekEvent({
        title: oswForm.title,
        description: oswForm.description || undefined,
        location: oswForm.location || undefined,
        status: oswForm.status as any,
        start_at,
        end_at,
      });
      setShowAddOswModal(false);
      setOswForm({
        title: '',
        description: '',
        location: '',
        status: 'upcoming',
        startDate: '',
        startTime: '00:00',
        endDate: '',
        endTime: '00:00',
      });
      await fetchOswEvents();
    } catch (err) {
      setErrorMessage(err instanceof Error ? err.message : 'Failed to create event.');
    } finally {
      setIsSubmitting(false);
    }
  };

  const confirmDelete = (id: string, name: string) => {
    setDeleteConfirm({ id, name });
  };

  const handleDeleteConfirmed = async () => {
    if (!deleteConfirm) return;
    const { id } = deleteConfirm;
    setDeletingId(id);
    try {
      setErrorMessage(null);
      await deleteEcosystem(id);
      await fetchEcosystems();
      window.dispatchEvent(new CustomEvent('ecosystems-updated'));
      setDeleteConfirm(null);
    } catch (error) {
      console.error('Failed to delete ecosystem:', error);
      setErrorMessage(error instanceof Error ? error.message : 'Failed to delete ecosystem.');
    } finally {
      setDeletingId(null);
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const nameError = validateName(formData.name);
    const descError = validateDescription(formData.description);
    const urlError = validateWebsiteUrl(formData.websiteUrl);

    const newErrors: Record<string, string> = {};
    if (nameError) newErrors.name = nameError;
    if (descError) newErrors.description = descError;
    if (urlError) newErrors.websiteUrl = urlError;

    setErrors(newErrors);
    if (Object.keys(newErrors).length > 0) return;

    setIsSubmitting(true);
    try {
      setErrorMessage(null);
      await createEcosystem({
        name: formData.name,
        description: formData.description || undefined,
        website_url: formData.websiteUrl || undefined,
        status: formData.status as 'active' | 'inactive',
      });
      setShowAddModal(false);
      setFormData({ name: '', description: '', status: 'active', websiteUrl: '' });
      await fetchEcosystems();
      window.dispatchEvent(new CustomEvent('ecosystems-updated'));
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : 'Failed to create ecosystem.');
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="space-y-10 pb-20">
      {/* Admin Header */}
      <div className={`backdrop-blur-[40px] bg-gradient-to-br rounded-[28px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-10 transition-all overflow-hidden relative ${theme === 'dark'
        ? 'from-white/[0.08] to-white/[0.04] border-white/10'
        : 'from-white/[0.15] to-white/[0.08] border-white/20'
        }`}>
        <div className="absolute -top-20 -right-20 w-80 h-80 bg-gradient-to-br from-[#c9983a]/20 to-transparent rounded-full blur-3xl"></div>
        <div className="relative z-10">
          <div className="flex items-start justify-between">
            <div className="flex-1">
              <div className="flex items-center gap-3 mb-3">
                <div className="p-2 rounded-[12px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] shadow-[0_6px_20px_rgba(162,121,44,0.35)] border border-white/10">
                  <Shield className="w-6 h-6 text-white" />
                </div>
                <h1 className={`text-[36px] font-bold transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                  }`}>Admin Panel</h1>
              </div>
              <p className={`text-[16px] max-w-3xl transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                }`}>
                Manage ecosystems, projects, and Open-Source Week events from a single dashboard.
              </p>
            </div>
            <div className={`px-4 py-2 rounded-[12px] backdrop-blur-[20px] border transition-colors ${theme === 'dark'
              ? 'bg-white/[0.08] border-white/15 text-[#d4d4d4]'
              : 'bg-white/[0.15] border-white/25 text-[#7a6b5a]'
              }`}>
              <span className="text-[13px] font-medium">Admin Access Verified</span>
            </div>
          </div>
          <div className="flex items-center gap-4 mt-10">
            <button
              onClick={() => setActiveTab('ecosystems')}
              className={`px-6 py-2.5 rounded-[14px] text-[14px] font-bold transition-all ${activeTab === 'ecosystems'
                ? 'bg-[#c9983a] text-white shadow-lg scale-105'
                : 'bg-white/5 hover:bg-white/10 text-[#7a6b5a] border border-white/10'
                }`}
            >
              Ecosystems
            </button>
            <button
              onClick={() => setActiveTab('projects')}
              className={`px-6 py-2.5 rounded-[14px] text-[14px] font-bold transition-all ${activeTab === 'projects'
                ? 'bg-[#c9983a] text-white shadow-lg scale-105'
                : 'bg-white/5 hover:bg-white/10 text-[#7a6b5a] border border-white/10'
                }`}
            >
              Projects
            </button>
            <button
              onClick={() => setActiveTab('events')}
              className={`px-6 py-2.5 rounded-[14px] text-[14px] font-bold transition-all ${activeTab === 'events'
                ? 'bg-[#c9983a] text-white shadow-lg scale-105'
                : 'bg-white/5 hover:bg-white/10 text-[#7a6b5a] border border-white/10'
                }`}
            >
              OSW Events
            </button>
          </div>
        </div>
      </div>

      {/* Global Error Message */}
      {errorMessage && (
        <div className={`rounded-[16px] border px-6 py-4 flex items-center justify-between ${theme === 'dark' ? 'bg-red-500/10 border-red-500/20 text-red-200' : 'bg-red-50 border-red-200 text-red-700'}`}>
          <p className="text-[14px]">{errorMessage}</p>
          <button onClick={() => setErrorMessage(null)} className="opacity-50 hover:opacity-100"><Plus className="w-4 h-4 rotate-45" /></button>
        </div>
      )}

      <div className="animate-in fade-in slide-in-from-bottom-4 duration-500">
        {activeTab === 'ecosystems' && (
          <section className={`backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8 transition-colors ${theme === 'dark'
            ? 'bg-white/[0.08] border-white/10'
            : 'bg-white/[0.15] border-white/20'
            }`}>
            <div className="flex items-center justify-between mb-8">
              <div>
                <h2 className={`text-[24px] font-bold mb-2 transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                  }`}>Ecosystem Management</h2>
                <p className={`text-[14px] transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                  }`}>Configure and curate technology ecosystems</p>
              </div>
              <button
                onClick={() => setShowAddModal(true)}
                className="flex items-center gap-2 px-6 py-3 bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white rounded-[16px] font-semibold text-[14px] shadow-lg hover:scale-105 transition-all"
              >
                <Plus className="w-5 h-5" />
                Add Ecosystem
              </button>
            </div>

            {isLoading ? (
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6 animate-pulse">
                {[...Array(3)].map((_, i) => (
                  <div key={i} className={`h-[220px] rounded-[20px] ${theme === 'dark' ? 'bg-white/5' : 'bg-black/5'}`} />
                ))}
              </div>
            ) : (
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                {ecosystems.map((eco) => (
                  <div key={eco.id} className={`p-6 rounded-[20px] border transition-all hover:shadow-xl ${theme === 'dark' ? 'bg-white/[0.04] border-white/10' : 'bg-white border-black/5 shadow-sm'}`}>
                    <div className="flex justify-between items-start mb-4">
                      <div className="w-12 h-12 rounded-[14px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] flex items-center justify-center text-white font-bold text-xl shadow-inner">
                        {eco.name.charAt(0)}
                      </div>
                      <button onClick={() => confirmDelete(eco.id, eco.name)} className="text-red-500 hover:bg-red-500/10 p-2.5 rounded-[12px] transition-colors">
                        <Trash2 className="w-5 h-5" />
                      </button>
                    </div>
                    <h3 className={`font-bold text-[20px] mb-2 ${theme === 'dark' ? 'text-white' : 'text-[#2d2820]'}`}>{eco.name}</h3>
                    <p className={`text-[14px] line-clamp-2 mb-4 ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>{eco.description}</p>
                    <div className="flex items-center gap-4 text-[12px] font-medium">
                      <span className={`px-2.5 py-1 rounded-full ${theme === 'dark' ? 'bg-white/10 text-[#d4d4d4]' : 'bg-black/5 text-[#7a6b5a]'}`}>{eco.project_count} Projects</span>
                      <span className={`px-2.5 py-1 rounded-full ${theme === 'dark' ? 'bg-green-500/10 text-green-400' : 'bg-green-50 text-green-700'}`}>{eco.status}</span>
                    </div>
                    {eco.project_count > 0 && (
                      <p className="mt-4 text-[11px] text-[#c9983a] font-medium">⚠️ Ecosystem has projects and cannot be deleted.</p>
                    )}
                  </div>
                ))}
              </div>
            )}
          </section>
        )}

        {activeTab === 'projects' && (
          <section className={`backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8 transition-colors ${theme === 'dark'
            ? 'bg-white/[0.08] border-white/10'
            : 'bg-white/[0.15] border-white/20'
            }`}>
            <div className="flex items-center justify-between mb-8">
              <div>
                <h2 className={`text-[24px] font-bold mb-2 transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                  }`}>Project Management</h2>
                <p className={`text-[14px] transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                  }`}>Review and manage all repositories registered on Grainlify</p>
              </div>
              <button
                onClick={() => setShowAddProjectModal(true)}
                className="flex items-center gap-2 px-6 py-3 bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white rounded-[16px] font-semibold text-[14px] shadow-lg hover:scale-105 transition-all"
              >
                <Plus className="w-5 h-5" />
                Add Project
              </button>
            </div>

            {isAdminProjectsLoading ? (
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-6 animate-pulse">
                {[...Array(4)].map((_, i) => (
                  <div key={i} className={`h-[320px] rounded-[20px] ${theme === 'dark' ? 'bg-white/5' : 'bg-black/5'}`} />
                ))}
              </div>
            ) : adminProjects.length === 0 ? (
              <div className={`text-center py-20 rounded-[20px] border-2 border-dashed ${theme === 'dark' ? 'border-white/10 text-[#7a6b5a]' : 'border-black/5 text-[#7a6b5a]'}`}>
                <Package className="w-12 h-12 mx-auto mb-4 opacity-20" />
                <p className="font-medium text-[16px]">No projects have been added yet.</p>
              </div>
            ) : (
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-6">
                {adminProjects.map((project) => (
                  <ProjectCard
                    key={project.id}
                    project={project}
                    showDelete={true}
                    onDelete={confirmDeleteProject}
                  />
                ))}
              </div>
            )}
          </section>
        )}

        {activeTab === 'events' && (
          <section className={`backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8 transition-colors ${theme === 'dark'
            ? 'bg-white/[0.08] border-white/10'
            : 'bg-white/[0.15] border-white/20'
            }`}>
            <div className="flex items-center justify-between mb-8">
              <div>
                <h2 className={`text-[24px] font-bold mb-2 transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                  }`}>OSW Events</h2>
                <p className={`text-[14px] transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                  }`}>Manage upcoming Open-Source Week events</p>
              </div>
              <button
                onClick={() => setShowAddOswModal(true)}
                className="flex items-center gap-2 px-6 py-3 bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white rounded-[16px] font-semibold text-[14px] shadow-lg hover:scale-105 transition-all"
              >
                <Calendar className="w-5 h-5" />
                Create Event
              </button>
            </div>

            {isOswLoading ? (
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6 animate-pulse">
                {[...Array(3)].map((_, i) => (
                  <div key={i} className={`h-[180px] rounded-[20px] ${theme === 'dark' ? 'bg-white/5' : 'bg-black/5'}`} />
                ))}
              </div>
            ) : (
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                {oswEvents.map((ev) => (
                  <div key={ev.id} className={`p-6 rounded-[20px] border relative ${theme === 'dark' ? 'bg-white/[0.04] border-white/10' : 'bg-white border-black/5'}`}>
                    <div className="flex justify-between items-start mb-4">
                      <div>
                        <h3 className={`font-bold text-[18px] ${theme === 'dark' ? 'text-white' : 'text-[#2d2820]'}`}>{ev.title}</h3>
                        <p className={`text-[13px] font-medium mt-1 ${theme === 'dark' ? 'text-[#c9983a]' : 'text-[#a67c2e]'}`}>{new Date(ev.start_at).toLocaleDateString()} - {new Date(ev.end_at).toLocaleDateString()}</p>
                      </div>
                      <button onClick={() => confirmDeleteOsw(ev.id, ev.title)} className="text-red-500 hover:bg-red-500/10 p-2.5 rounded-[12px] transition-colors">
                        <Trash2 className="w-5 h-5" />
                      </button>
                    </div>
                    <div className="flex items-center gap-2 mt-4">
                      <span className={`px-3 py-1 rounded-full text-[11px] font-bold uppercase tracking-wider ${theme === 'dark' ? 'bg-white/10 text-white' : 'bg-black/5 text-black'}`}>{ev.status}</span>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </section>
        )}
      </div>

      {/* Modals */}
      <Modal isOpen={showAddModal} onClose={() => setShowAddModal(false)} title="Add New Ecosystem">
        <form onSubmit={handleSubmit} className="space-y-6">
          <ModalInput label="Name" value={formData.name} onChange={(v) => setFormData({ ...formData, name: v })} placeholder="e.g. Web3, AI, Robotics" required />
          <ModalInput label="Description" value={formData.description} onChange={(v) => setFormData({ ...formData, description: v })} placeholder="Global community for AI research..." rows={3} required />
          <ModalInput label="Website URL" value={formData.websiteUrl} onChange={(v) => setFormData({ ...formData, websiteUrl: v })} placeholder="https://ecosystem.org" />
          <ModalFooter>
            <ModalButton onClick={() => setShowAddModal(false)}>Cancel</ModalButton>
            <ModalButton type="submit" variant="primary" disabled={isSubmitting}>{isSubmitting ? 'Creating...' : 'Create Ecosystem'}</ModalButton>
          </ModalFooter>
        </form>
      </Modal>

      <Modal isOpen={showAddOswModal} onClose={() => setShowAddOswModal(false)} title="Create OSW Event">
        <form onSubmit={handleCreateOsw} className="space-y-6">
          <ModalInput label="Title" value={oswForm.title} onChange={(v) => setOswForm({ ...oswForm, title: v })} placeholder="Open-Source Week 2026" required />
          <div className="grid grid-cols-2 gap-4">
            <ModalInput label="Start Date" type="date" value={oswForm.startDate} onChange={(v) => setOswForm({ ...oswForm, startDate: v })} required />
            <ModalInput label="End Date" type="date" value={oswForm.endDate} onChange={(v) => setOswForm({ ...oswForm, endDate: v })} required />
          </div>
          <ModalFooter>
            <ModalButton onClick={() => setShowAddOswModal(false)}>Cancel</ModalButton>
            <ModalButton type="submit" variant="primary" disabled={isSubmitting}>{isSubmitting ? 'Creating...' : 'Create Event'}</ModalButton>
          </ModalFooter>
        </form>
      </Modal>
      <Modal isOpen={showAddProjectModal} onClose={() => setShowAddProjectModal(false)} title="Add New Project">
        <form onSubmit={handleCreateProject} className="space-y-6">
          <ModalInput
            label="GitHub Repository"
            value={projectFormData.github_full_name}
            onChange={(v) => setProjectFormData({ ...projectFormData, github_full_name: v })}
            placeholder="e.g. facebook/react"
            required
          />
          <ModalSelect
            label="Ecosystem"
            value={projectFormData.ecosystem_name}
            onChange={(v) => setProjectFormData({ ...projectFormData, ecosystem_name: v })}
            options={ecosystems.map(e => ({ value: e.name, label: e.name }))}
            required
          />
          <div className="grid grid-cols-2 gap-4">
            <ModalSelect
              label="Category"
              value={projectFormData.category}
              onChange={(v) => setProjectFormData({ ...projectFormData, category: v })}
              options={[
                { value: 'defi', label: 'DeFi' },
                { value: 'nft', label: 'NFT' },
                { value: 'infrastructure', label: 'Infrastructure' },
                { value: 'tooling', label: 'Tooling' },
                { value: 'gaming', label: 'Gaming' },
                { value: 'dao', label: 'DAO' },
                { value: 'other', label: 'Other' },
              ]}
              required
            />
            <ModalInput
              label="Primary Language"
              value={projectFormData.language}
              onChange={(v) => setProjectFormData({ ...projectFormData, language: v })}
              placeholder="e.g. TypeScript, Rust"
            />
          </div>
          <ModalInput
            label="Tags (comma separated)"
            value={projectFormData.tags}
            onChange={(v) => setProjectFormData({ ...projectFormData, tags: v })}
            placeholder="e.g. react, web3, wallet"
          />
          <ModalFooter>
            <ModalButton onClick={() => setShowAddProjectModal(false)}>Cancel</ModalButton>
            <ModalButton type="submit" variant="primary" disabled={isSubmitting}>{isSubmitting ? 'Adding...' : 'Add Project'}</ModalButton>
          </ModalFooter>
        </form>
      </Modal>
      {/* Confirmation Modals */}
      <Modal isOpen={!!deleteConfirm} onClose={() => setDeleteConfirm(null)} title="Delete Ecosystem">
        <div className="px-1 py-2">
          <p className={`text-[15px] mb-6 ${theme === 'dark' ? 'text-white' : 'text-[#2d2820]'}`}>Are you sure you want to delete <span className="font-bold text-[#c9983a]">"{deleteConfirm?.name}"</span>? This action cannot be undone.</p>
          <ModalFooter>
            <ModalButton variant="secondary" onClick={() => setDeleteConfirm(null)}>Cancel</ModalButton>
            <ModalButton variant="primary" onClick={handleDeleteConfirmed} disabled={!!deletingId}>{deletingId ? 'Deleting...' : 'Delete'}</ModalButton>
          </ModalFooter>
        </div>
      </Modal>

      <Modal isOpen={!!projectDeleteConfirm} onClose={() => setProjectDeleteConfirm(null)} title="Remove Project">
        <div className="px-1 py-2">
          <p className={`text-[15px] mb-6 ${theme === 'dark' ? 'text-white' : 'text-[#2d2820]'}`}>Are you sure you want to remove <span className="font-bold text-[#c9983a]">"{projectDeleteConfirm?.name}"</span> from the platform? This will stop all synchronization.</p>
          <ModalFooter>
            <ModalButton variant="secondary" onClick={() => setProjectDeleteConfirm(null)}>Cancel</ModalButton>
            <ModalButton variant="primary" onClick={handleDeleteProjectConfirmed} disabled={isDeletingProject}>{isDeletingProject ? 'Removing...' : 'Remove Project'}</ModalButton>
          </ModalFooter>
        </div>
      </Modal>

      <Modal isOpen={!!oswDeleteConfirm} onClose={() => setOswDeleteConfirm(null)} title="Delete Event">
        <div className="px-1 py-2">
          <p className={`text-[15px] mb-6 ${theme === 'dark' ? 'text-white' : 'text-[#2d2820]'}`}>Are you sure you want to delete <span className="font-bold text-[#c9983a]">"{oswDeleteConfirm?.title}"</span>?</p>
          <ModalFooter>
            <ModalButton variant="secondary" onClick={() => setOswDeleteConfirm(null)}>Cancel</ModalButton>
            <ModalButton variant="primary" onClick={handleDeleteOswConfirmed} disabled={!!oswDeletingId}>{oswDeletingId ? 'Deleting...' : 'Delete'}</ModalButton>
          </ModalFooter>
        </div>
      </Modal>
    </div>
  );
}