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
  deleteAdminProject
} from '../../../shared/api/client';
import { ProjectCard, Project } from '../../dashboard/components/ProjectCard';

// Helper functions (copied from BrowsePage for consistency)
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
  const [activeTab, setActiveTab] = useState<'ecosystems' | 'projects' | 'osw'>('ecosystems');
  const [showAddModal, setShowAddModal] = useState(false);
  const [ecosystems, setEcosystems] = useState<Ecosystem[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [deleteConfirm, setDeleteConfirm] = useState<{ id: string; name: string } | null>(null);
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

  const fetchAdminProjects = useCallback(async () => {
    try {
      setIsAdminProjectsLoading(true);
      const response = await getAdminProjects();

      const mappedProjects: Project[] = (response.projects || []).map((p: any) => ({
        id: p.id,
        name: p.github_full_name.split('/')[1] || p.github_full_name,
        icon: getProjectIcon(p.github_full_name),
        stars: formatNumber(p.stars_count || 0),
        forks: formatNumber(p.forks_count || 0),
        contributors: p.contributors_count || 0,
        openIssues: p.open_issues_count || 0,
        prs: p.open_prs_count || 0,
        description: truncateDescription(p.description) || `${p.language || 'Project'} repository`,
        tags: Array.isArray(p.tags) ? p.tags : [],
        color: getProjectColor(p.github_full_name.split('/')[1] || p.github_full_name),
      }));

      setAdminProjects(mappedProjects);
    } catch (error) {
      console.error('Failed to fetch admin projects:', error);
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

    const handleEcosystemsUpdated = () => {
      fetchEcosystems();
    };
    const handleProjectsUpdated = () => {
      fetchAdminProjects();
    };
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

  const confirmDeleteProject = (_e: React.MouseEvent, id: string, name: string) => {
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
    <div className="space-y-6">
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
                Manage ecosystems, review requests, and oversee platform operations.
              </p>
            </div>
            <div className="flex items-center gap-3">
              <div className={`px-4 py-2 rounded-[12px] backdrop-blur-[20px] border transition-colors ${theme === 'dark'
                ? 'bg-white/[0.08] border-white/15 text-[#d4d4d4]'
                : 'bg-white/[0.15] border-white/25 text-[#7a6b5a]'
                }`}>
                <span className="text-[13px] font-medium">Admin Access</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Admin Tabs */}
      <div className={`flex items-center gap-2 p-1.5 rounded-[18px] backdrop-blur-[20px] border shadow-sm w-fit ${theme === 'dark'
        ? 'bg-white/[0.05] border-white/10'
        : 'bg-white/[0.1] border-white/20'
        }`}>
        {[
          { id: 'ecosystems', label: 'Ecosystems', icon: Globe },
          { id: 'projects', label: 'Projects', icon: Package },
          { id: 'osw', label: 'OSW Events', icon: Calendar },
        ].map((tab) => {
          const isActive = activeTab === tab.id;
          const Icon = tab.icon;
          return (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id as any)}
              className={`flex items-center gap-2 px-6 py-2.5 rounded-[14px] text-[14px] font-semibold transition-all duration-300 ${isActive
                ? 'bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white shadow-[0_4px_12px_rgba(162,121,44,0.3)]'
                : theme === 'dark'
                  ? 'text-[#d4d4d4] hover:bg-white/10'
                  : 'text-[#7a6b5a] hover:bg-white/20'
                }`}
            >
              <Icon className="w-4 h-4" />
              {tab.label}
            </button>
          );
        })}
      </div>

      {/* Tab Content */}
      <div className="mt-8">
        {activeTab === 'ecosystems' && (
          <div className="space-y-6">
            <div className="flex items-center justify-between">
              <div>
                <h2 className={`text-[24px] font-bold mb-1 transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'}`}>Ecosystem Management</h2>
                <p className={`text-[14px] transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>Add or remove ecosystems from the platform</p>
              </div>
              <button
                onClick={() => setShowAddModal(true)}
                className="flex items-center gap-2 px-5 py-2.5 bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white rounded-[12px] font-semibold text-[14px]"
              >
                <Plus className="w-4 h-4" />
                Add Ecosystem
              </button>
            </div>

            {isLoading ? (
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6 animate-pulse">
                {[...Array(3)].map((_, i) => (
                  <div key={i} className={`h-[200px] rounded-[16px] ${theme === 'dark' ? 'bg-white/5' : 'bg-black/5'}`} />
                ))}
              </div>
            ) : (
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                {ecosystems.map((eco) => (
                  <div key={eco.id} className={`p-5 rounded-[16px] border transition-all ${theme === 'dark' ? 'bg-white/[0.04] border-white/10' : 'bg-white border-black/5'}`}>
                    <div className="flex justify-between items-start mb-4">
                      <div className="w-10 h-10 rounded-[10px] bg-[#c9983a]/20 flex items-center justify-center text-[#c9983a] font-bold">
                        {eco.name.charAt(0)}
                      </div>
                      <button onClick={() => confirmDelete(eco.id, eco.name)} className="text-red-500 hover:bg-red-500/10 p-2 rounded-[8px]">
                        <Trash2 className="w-4 h-4" />
                      </button>
                    </div>
                    <h3 className={`font-bold text-[18px] mb-1 ${theme === 'dark' ? 'text-white' : 'text-[#2d2820]'}`}>{eco.name}</h3>
                    <p className={`text-[13px] line-clamp-2 ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>{eco.description}</p>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {activeTab === 'projects' && (
          <div className="space-y-6">
            <div>
              <h2 className={`text-[24px] font-bold mb-1 transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'}`}>Project Management</h2>
              <p className={`text-[14px] transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>Manage all projects registered on the platform</p>
            </div>

            {isAdminProjectsLoading ? (
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-6 animate-pulse">
                {[...Array(4)].map((_, i) => (
                  <div key={i} className={`h-[300px] rounded-[18px] ${theme === 'dark' ? 'bg-white/5' : 'bg-black/5'}`} />
                ))}
              </div>
            ) : adminProjects.length === 0 ? (
              <div className="text-center py-20 opacity-50">No projects found.</div>
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
          </div>
        )}

        {activeTab === 'osw' && (
          <div className="space-y-6">
            <div className="flex items-center justify-between">
              <div>
                <h2 className={`text-[24px] font-bold mb-1 transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'}`}>OSW Events</h2>
                <p className={`text-[14px] transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>Manage Open-Source Week events</p>
              </div>
              <button
                onClick={() => setShowAddOswModal(true)}
                className="flex items-center gap-2 px-5 py-2.5 bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white rounded-[12px] font-semibold text-[14px]"
              >
                <Plus className="w-4 h-4" />
                Add Event
              </button>
            </div>

            {isOswLoading ? (
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6 animate-pulse">
                {[...Array(3)].map((_, i) => (
                  <div key={i} className={`h-[150px] rounded-[16px] ${theme === 'dark' ? 'bg-white/5' : 'bg-black/5'}`} />
                ))}
              </div>
            ) : (
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                {oswEvents.map((ev) => (
                  <div key={ev.id} className={`p-5 rounded-[16px] border ${theme === 'dark' ? 'bg-white/[0.04] border-white/10' : 'bg-white border-black/5'}`}>
                    <div className="flex justify-between items-start mb-2">
                      <h3 className={`font-bold ${theme === 'dark' ? 'text-white' : 'text-[#2d2820]'}`}>{ev.title}</h3>
                      <button onClick={() => confirmDeleteOsw(ev.id, ev.title)} className="text-red-500 hover:bg-red-500/10 p-2 rounded-[8px]">
                        <Trash2 className="w-4 h-4" />
                      </button>
                    </div>
                    <p className={`text-[12px] mb-3 ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>{new Date(ev.start_at).toLocaleDateString()} - {new Date(ev.end_at).toLocaleDateString()}</p>
                    <span className="px-2 py-0.5 rounded-full bg-[#c9983a]/20 text-[#c9983a] text-[10px] font-bold uppercase tracking-wider">{ev.status}</span>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </div>

      {/* Modals */}
      <Modal isOpen={showAddModal} onClose={() => setShowAddModal(false)} title="Add Ecosystem">
        <form onSubmit={handleSubmit} className="space-y-4">
          <ModalInput label="Name" value={formData.name} onChange={(v) => setFormData({ ...formData, name: v })} required />
          <ModalInput label="Description" value={formData.description} onChange={(v) => setFormData({ ...formData, description: v })} rows={3} />
          <ModalInput label="Website URL" value={formData.websiteUrl} onChange={(v) => setFormData({ ...formData, websiteUrl: v })} />
          <ModalFooter>
             <ModalButton onClick={() => setShowAddModal(false)}>Cancel</ModalButton>
             <ModalButton type="submit" variant="primary">Add Ecosystem</ModalButton>
          </ModalFooter>
        </form>
      </Modal>

      <Modal isOpen={showAddOswModal} onClose={() => setShowAddOswModal(false)} title="Add OSW Event">
        <form onSubmit={handleCreateOsw} className="space-y-4">
          <ModalInput label="Title" value={oswForm.title} onChange={(v) => setOswForm({ ...oswForm, title: v })} required />
          <div className="grid grid-cols-2 gap-4">
            <ModalInput label="Start Date" type="date" value={oswForm.startDate} onChange={(v) => setOswForm({ ...oswForm, startDate: v })} required />
            <ModalInput label="End Date" type="date" value={oswForm.endDate} onChange={(v) => setOswForm({ ...oswForm, endDate: v })} required />
          </div>
          <ModalFooter>
             <ModalButton onClick={() => setShowAddOswModal(false)}>Cancel</ModalButton>
             <ModalButton type="submit" variant="primary">Create Event</ModalButton>
          </ModalFooter>
        </form>
      </Modal>

      <Modal isOpen={!!deleteConfirm} onClose={() => setDeleteConfirm(null)} title="Delete Ecosystem">
        <div className="p-4">
          <p>Are you sure you want to delete {deleteConfirm?.name}?</p>
          <ModalFooter>
            <ModalButton onClick={() => setDeleteConfirm(null)}>Cancel</ModalButton>
            <ModalButton variant="primary" onClick={handleDeleteConfirmed}>Delete</ModalButton>
          </ModalFooter>
        </div>
      </Modal>

      <Modal isOpen={!!projectDeleteConfirm} onClose={() => setProjectDeleteConfirm(null)} title="Delete Project">
        <div className="p-4">
          <p>Are you sure you want to delete {projectDeleteConfirm?.name}?</p>
          <ModalFooter>
            <ModalButton onClick={() => setProjectDeleteConfirm(null)}>Cancel</ModalButton>
            <ModalButton variant="primary" onClick={handleDeleteProjectConfirmed}>Delete Project</ModalButton>
          </ModalFooter>
        </div>
      </Modal>

      <Modal isOpen={!!oswDeleteConfirm} onClose={() => setOswDeleteConfirm(null)} title="Delete Event">
        <div className="p-4">
          <p>Are you sure you want to delete {oswDeleteConfirm?.title}?</p>
          <ModalFooter>
            <ModalButton onClick={() => setOswDeleteConfirm(null)}>Cancel</ModalButton>
            <ModalButton variant="primary" onClick={handleDeleteOswConfirmed}>Delete</ModalButton>
          </ModalFooter>
        </div>
      </Modal>
    </div>
  );
}