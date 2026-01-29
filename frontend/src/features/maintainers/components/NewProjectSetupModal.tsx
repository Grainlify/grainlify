import { useState, useEffect } from 'react';
import { X, Loader2, AlertCircle, CheckCircle2 } from 'lucide-react';
import { useTheme } from '../../../shared/contexts/ThemeContext';
import { updateProjectMetadata, getEcosystems } from '../../../shared/api/client';
import { SkeletonLoader } from '../../../shared/components/SkeletonLoader';

export interface PendingProject {
  id: string;
  github_full_name: string;
  ecosystem_name: string | null;
}

interface NewProjectSetupModalProps {
  isOpen: boolean;
  project: PendingProject | null;
  onClose: () => void;
  onSaved: () => void;
  onSkip: () => void;
}

export function NewProjectSetupModal({
  isOpen,
  project,
  onClose,
  onSaved,
  onSkip,
}: NewProjectSetupModalProps) {
  const { theme } = useTheme();
  const darkTheme = theme === 'dark';

  const [description, setDescription] = useState('');
  const [ecosystemName, setEcosystemName] = useState('');
  const [tags, setTags] = useState('');
  const [category, setCategory] = useState('');

  const [ecosystems, setEcosystems] = useState<Array<{ name: string; slug: string }>>([]);
  const [isLoadingEcosystems, setIsLoadingEcosystems] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState(false);

  useEffect(() => {
    if (isOpen) {
      loadEcosystems();
    }
  }, [isOpen]);

  useEffect(() => {
    if (project) {
      setEcosystemName(project.ecosystem_name ?? '');
      setDescription('');
      setTags('');
      setCategory('');
      setError(null);
      setSuccess(false);
    }
  }, [project?.id]);

  const loadEcosystems = async () => {
    setIsLoadingEcosystems(true);
    setError(null);
    try {
      const data = await getEcosystems();
      setEcosystems(data.ecosystems.map((eco: { name: string; slug: string }) => ({ name: eco.name, slug: eco.slug })));
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load ecosystems');
    } finally {
      setIsLoadingEcosystems(false);
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!project) return;
    setError(null);
    setSuccess(false);

    if (!ecosystemName.trim()) {
      setError('Ecosystem is required');
      return;
    }

    setIsSubmitting(true);

    try {
      const tagsArray = tags
        .split(',')
        .map((tag) => tag.trim())
        .filter((tag) => tag.length > 0);

      await updateProjectMetadata(project.id, {
        description: description.trim() || undefined,
        ecosystem_name: ecosystemName.trim(),
        tags: tagsArray.length > 0 ? tagsArray : undefined,
        category: category.trim() || undefined,
      });

      setSuccess(true);
      setTimeout(() => {
        onSaved();
        setSuccess(false);
      }, 800);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to save project details');
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleSkip = () => {
    if (!isSubmitting) {
      setError(null);
      onSkip();
      onClose();
    }
  };

  if (!isOpen || !project) return null;

  return (
    <div className="fixed inset-0 z-[100] flex items-center justify-center p-4">
      <div className="absolute inset-0 bg-black/50 backdrop-blur-sm" onClick={handleSkip} />

      <div
        className={`relative w-full max-w-[520px] rounded-[24px] border-2 shadow-[0_16px_64px_rgba(0,0,0,0.4)] transition-colors ${
          darkTheme ? 'bg-[#3a3228] border-white/30' : 'bg-[#d4c5b0] border-white/40'
        }`}
        onClick={(e) => e.stopPropagation()}
      >
        <div
          className={`flex items-center justify-between px-6 py-5 border-b-2 transition-colors ${
            darkTheme ? 'border-white/20' : 'border-white/30'
          }`}
        >
          <h2
            className={`text-[20px] font-bold transition-colors ${
              darkTheme ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
            }`}
          >
            New Project Setup
          </h2>
          <button
            onClick={handleSkip}
            disabled={isSubmitting}
            className={`p-2 rounded-[10px] transition-all ${
              darkTheme
                ? 'hover:bg-white/10 text-[#b8a898] hover:text-[#e8dfd0]'
                : 'hover:bg-white/20 text-[#7a6b5a] hover:text-[#2d2820]'
            } ${isSubmitting ? 'opacity-50 cursor-not-allowed' : ''}`}
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        <div className="px-6 py-4 border-b border-white/10">
          <p
            className={`text-[13px] font-semibold ${
              darkTheme ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
            }`}
          >
            {project.github_full_name}
          </p>
          <p className={`text-[12px] mt-0.5 ${darkTheme ? 'text-[#b8a898]' : 'text-[#7a6b5a]'}`}>
            Add a short description and ecosystem so others can discover this project.
          </p>
        </div>

        <form onSubmit={handleSubmit} className="p-6 space-y-5">
          {error && (
            <div
              className={`flex items-center gap-3 p-4 rounded-[12px] border-2 ${
                darkTheme
                  ? 'bg-red-500/10 border-red-500/30 text-red-400'
                  : 'bg-red-100 border-red-300 text-red-700'
              }`}
            >
              <AlertCircle className="w-5 h-5 flex-shrink-0" />
              <span className="text-[14px] font-medium">{error}</span>
            </div>
          )}

          {success && (
            <div
              className={`flex items-center gap-3 p-4 rounded-[12px] border-2 ${
                darkTheme
                  ? 'bg-green-500/10 border-green-500/30 text-green-400'
                  : 'bg-green-100 border-green-300 text-green-700'
              }`}
            >
              <CheckCircle2 className="w-5 h-5 flex-shrink-0" />
              <span className="text-[14px] font-medium">Project details saved!</span>
            </div>
          )}

          <div>
            <label
              className={`block text-[14px] font-semibold mb-2 transition-colors ${
                darkTheme ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
              }`}
            >
              Description
            </label>
            <textarea
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="What does this project do? Who is it for?"
              disabled={isSubmitting}
              rows={3}
              className={`w-full px-4 py-3 rounded-[12px] border-2 transition-all resize-none ${
                darkTheme
                  ? 'bg-white/10 border-white/20 text-[#e8dfd0] placeholder:text-[#b8a898] focus:border-[#c9983a] focus:bg-white/15'
                  : 'bg-white/40 border-white/50 text-[#2d2820] placeholder:text-[#7a6b5a] focus:border-[#c9983a] focus:bg-white/60'
              } ${isSubmitting ? 'opacity-50 cursor-not-allowed' : ''}`}
            />
          </div>

          <div>
            <label
              className={`block text-[14px] font-semibold mb-2 transition-colors ${
                darkTheme ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
              }`}
            >
              Ecosystem <span className="text-red-500">*</span>
            </label>
            {isLoadingEcosystems ? (
              <SkeletonLoader className="h-12 w-full rounded-[12px]" />
            ) : (
              <select
                value={ecosystemName}
                onChange={(e) => setEcosystemName(e.target.value)}
                disabled={isSubmitting || ecosystems.length === 0}
                className={`w-full px-4 py-3 rounded-[12px] border-2 transition-all ${
                  darkTheme
                    ? 'bg-white/10 border-white/20 text-[#e8dfd0] focus:border-[#c9983a] focus:bg-white/15'
                    : 'bg-white/40 border-white/50 text-[#2d2820] focus:border-[#c9983a] focus:bg-white/60'
                } ${isSubmitting || ecosystems.length === 0 ? 'opacity-50 cursor-not-allowed' : ''}`}
                required
              >
                <option value="">Select an ecosystem</option>
                {ecosystems.map((eco) => (
                  <option key={eco.slug} value={eco.name}>
                    {eco.name}
                  </option>
                ))}
              </select>
            )}
          </div>

          <div>
            <label
              className={`block text-[14px] font-semibold mb-2 transition-colors ${
                darkTheme ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
              }`}
            >
              Tags (Optional)
            </label>
            <input
              type="text"
              value={tags}
              onChange={(e) => setTags(e.target.value)}
              placeholder="Comma-separated: good first issue, help wanted"
              disabled={isSubmitting}
              className={`w-full px-4 py-3 rounded-[12px] border-2 transition-all ${
                darkTheme
                  ? 'bg-white/10 border-white/20 text-[#e8dfd0] placeholder:text-[#b8a898] focus:border-[#c9983a] focus:bg-white/15'
                  : 'bg-white/40 border-white/50 text-[#2d2820] placeholder:text-[#7a6b5a] focus:border-[#c9983a] focus:bg-white/60'
              } ${isSubmitting ? 'opacity-50 cursor-not-allowed' : ''}`}
            />
          </div>

          <div>
            <label
              className={`block text-[14px] font-semibold mb-2 transition-colors ${
                darkTheme ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
              }`}
            >
              Category (Optional)
            </label>
            <input
              type="text"
              value={category}
              onChange={(e) => setCategory(e.target.value)}
              placeholder="e.g., Frontend, Backend, Full Stack"
              disabled={isSubmitting}
              className={`w-full px-4 py-3 rounded-[12px] border-2 transition-all ${
                darkTheme
                  ? 'bg-white/10 border-white/20 text-[#e8dfd0] placeholder:text-[#b8a898] focus:border-[#c9983a] focus:bg-white/15'
                  : 'bg-white/40 border-white/50 text-[#2d2820] placeholder:text-[#7a6b5a] focus:border-[#c9983a] focus:bg-white/60'
              } ${isSubmitting ? 'opacity-50 cursor-not-allowed' : ''}`}
            />
          </div>

          <div className="flex items-center gap-3 pt-2">
            <button
              type="button"
              onClick={handleSkip}
              disabled={isSubmitting}
              className={`flex-1 px-5 py-3 rounded-[12px] border-2 font-semibold text-[14px] transition-all ${
                darkTheme
                  ? 'bg-white/10 border-white/20 text-[#e8dfd0] hover:bg-white/15'
                  : 'bg-white/40 border-white/50 text-[#2d2820] hover:bg-white/60'
              } ${isSubmitting ? 'opacity-50 cursor-not-allowed' : ''}`}
            >
              Skip for now
            </button>
            <button
              type="submit"
              disabled={isSubmitting || success}
              className={`flex-1 px-5 py-3 rounded-[12px] border-2 font-semibold text-[14px] transition-all ${
                darkTheme
                  ? 'bg-gradient-to-br from-[#c9983a]/40 to-[#d4af37]/30 border-[#c9983a]/70 text-[#fef5e7] hover:from-[#c9983a]/50 hover:to-[#d4af37]/40 shadow-[0_4px_16px_rgba(201,152,58,0.4)]'
                  : 'bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/25 border-[#c9983a]/50 text-[#2d2820] hover:from-[#c9983a]/40 hover:to-[#d4af37]/35 shadow-[0_4px_16px_rgba(201,152,58,0.25)]'
              } ${isSubmitting || success ? 'opacity-50 cursor-not-allowed' : ''}`}
            >
              {isSubmitting ? (
                <span className="flex items-center justify-center gap-2">
                  <Loader2 className="w-4 h-4 animate-spin" />
                  Saving...
                </span>
              ) : success ? (
                'Saved!'
              ) : (
                'Save & Continue'
              )}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
