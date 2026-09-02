import { useState } from "react";

interface OnboardingProps {
  onComplete: () => void;
  port: number;
}

type Step = "welcome" | "import_repo" | "add_account" | "launch";

export function Onboarding({ onComplete, port }: OnboardingProps) {
  const [step, setStep] = useState<Step>("welcome");
  const [repoPath, setRepoPath] = useState("");
  const [provider, setProvider] = useState("claude");
  const [accountLabel, setAccountLabel] = useState("");
  const [loading, setLoading] = useState(false);

  const handleImportRepo = async () => {
    if (!repoPath.trim()) return;
    setLoading(true);
    try {
      await fetch(`http://127.0.0.1:${port}/api/projects`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ path: repoPath.trim() }),
      });
      setStep("add_account");
    } catch {
      // API not ready yet — skip to next step for demo
      setStep("add_account");
    } finally {
      setLoading(false);
    }
  };

  const handleAddAccount = async () => {
    if (!accountLabel.trim()) return;
    setLoading(true);
    try {
      await fetch(`http://127.0.0.1:${port}/api/accounts`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ provider, label: accountLabel.trim() }),
      });
      setStep("launch");
    } catch {
      setStep("launch");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="h-full w-full flex items-center justify-center bg-surface">
      <div className="w-full max-w-md mx-4">
        {/* Progress indicator */}
        <div className="flex items-center justify-center gap-2 mb-8">
          {(["welcome", "import_repo", "add_account", "launch"] as Step[]).map((s, i) => (
            <div
              key={s}
              className={`w-2 h-2 rounded-full transition-colors ${
                step === s ? "bg-accent w-6" : i < ["welcome", "import_repo", "add_account", "launch"].indexOf(step) ? "bg-accent/50" : "bg-gray-600"
              }`}
            />
          ))}
        </div>

        <div className="bg-surface-raised rounded-xl border border-[var(--border-color)] p-8">
          {step === "welcome" && (
            <div className="text-center">
              <div className="w-16 h-16 rounded-2xl bg-accent/20 flex items-center justify-center mx-auto mb-4">
                <span className="text-2xl font-bold text-accent">IA</span>
              </div>
              <h1 className="text-xl font-bold text-gray-100 mb-2">Bem-vindo ao IA-Hub</h1>
              <p className="text-gray-400 text-sm mb-6">
                Multi-Agent IDE Control Plane. Vamos configurar seu ambiente em 3 passos rápidos.
              </p>
              <button
                onClick={() => setStep("import_repo")}
                className="w-full py-2.5 bg-accent hover:bg-accent/90 text-white rounded-lg text-sm font-medium transition-colors"
              >
                Começar
              </button>
            </div>
          )}

          {step === "import_repo" && (
            <div>
              <h2 className="text-lg font-bold text-gray-100 mb-1">Importar Repositório</h2>
              <p className="text-gray-400 text-sm mb-4">
                Selecione a pasta do projeto que você quer gerenciar com agentes de IA.
              </p>
              <input
                type="text"
                value={repoPath}
                onChange={(e) => setRepoPath(e.target.value)}
                placeholder="C:\Users\you\projects\my-app"
                className="w-full px-3 py-2 bg-surface border border-[var(--border-color)] rounded-lg text-sm text-gray-200 placeholder-gray-600 focus:outline-none focus:border-accent mb-4 font-mono"
              />
              <div className="flex gap-2">
                <button
                  onClick={() => setStep("welcome")}
                  className="flex-1 py-2 border border-[var(--border-color)] text-gray-400 rounded-lg text-sm hover:text-gray-200 transition-colors"
                >
                  Voltar
                </button>
                <button
                  onClick={handleImportRepo}
                  disabled={loading || !repoPath.trim()}
                  className="flex-1 py-2 bg-accent hover:bg-accent/90 disabled:opacity-50 text-white rounded-lg text-sm font-medium transition-colors"
                >
                  {loading ? "Importando..." : "Importar"}
                </button>
              </div>
            </div>
          )}

          {step === "add_account" && (
            <div>
              <h2 className="text-lg font-bold text-gray-100 mb-1">Adicionar Conta</h2>
              <p className="text-gray-400 text-sm mb-4">
                Conecte uma conta de provider de IA (Claude, Antigravity, Codex, etc.)
              </p>
              <select
                value={provider}
                onChange={(e) => setProvider(e.target.value)}
                className="w-full px-3 py-2 bg-surface border border-[var(--border-color)] rounded-lg text-sm text-gray-200 focus:outline-none focus:border-accent mb-3"
              >
                <option value="claude">Claude Code</option>
                <option value="antigravity">Antigravity</option>
                <option value="codex">Codex</option>
                <option value="opencode">OpenCode</option>
              </select>
              <input
                type="text"
                value={accountLabel}
                onChange={(e) => setAccountLabel(e.target.value)}
                placeholder="Minha conta Claude A"
                className="w-full px-3 py-2 bg-surface border border-[var(--border-color)] rounded-lg text-sm text-gray-200 placeholder-gray-600 focus:outline-none focus:border-accent mb-4"
              />
              <div className="flex gap-2">
                <button
                  onClick={() => setStep("import_repo")}
                  className="flex-1 py-2 border border-[var(--border-color)] text-gray-400 rounded-lg text-sm hover:text-gray-200 transition-colors"
                >
                  Voltar
                </button>
                <button
                  onClick={handleAddAccount}
                  disabled={loading || !accountLabel.trim()}
                  className="flex-1 py-2 bg-accent hover:bg-accent/90 disabled:opacity-50 text-white rounded-lg text-sm font-medium transition-colors"
                >
                  {loading ? "Salvando..." : "Adicionar"}
                </button>
              </div>
            </div>
          )}

          {step === "launch" && (
            <div className="text-center">
              <div className="w-12 h-12 rounded-full bg-status-success/20 flex items-center justify-center mx-auto mb-4">
                <span className="text-status-success text-xl">✓</span>
              </div>
              <h2 className="text-lg font-bold text-gray-100 mb-2">Tudo Pronto!</h2>
              <p className="text-gray-400 text-sm mb-6">
                Seu ambiente está configurado. Você pode iniciar sessões de agentes a qualquer momento pelo painel principal.
              </p>
              <button
                onClick={onComplete}
                className="w-full py-2.5 bg-accent hover:bg-accent/90 text-white rounded-lg text-sm font-medium transition-colors"
              >
                Abrir Painel
              </button>
            </div>
          )}
        </div>

        {/* Skip link */}
        {step !== "launch" && (
          <div className="text-center mt-4">
            <button
              onClick={onComplete}
              className="text-gray-500 text-xs hover:text-gray-300 transition-colors"
            >
              Pular onboarding →
            </button>
          </div>
        )}
      </div>
    </div>
  );
}