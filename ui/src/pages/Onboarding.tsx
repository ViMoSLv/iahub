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

  const steps: Step[] = ["welcome", "import_repo", "add_account", "launch"];
  const currentIdx = steps.indexOf(step);

  return (
    <div className="h-full w-full flex items-center justify-center bg-[#0B0B0B]">
      <div className="w-full max-w-md mx-4">
        {/* Progress indicator */}
        <div className="flex items-center justify-center gap-2 mb-8">
          {steps.map((s, i) => (
            <div
              key={s}
              className={`h-1 rounded-full transition-all duration-300 ${
                step === s
                  ? "bg-[#007acc] w-8"
                  : i < currentIdx
                  ? "bg-[#007acc]/40 w-2"
                  : "bg-[#333] w-2"
              }`}
            />
          ))}
        </div>

        <div className="bg-[#121212] rounded-xl border border-[#232323] p-8">
          {step === "welcome" && (
            <div className="text-center">
              <div className="w-16 h-16 rounded-2xl bg-[#007acc]/15 flex items-center justify-center mx-auto mb-4 border border-[#007acc]/20">
                <span className="text-2xl font-bold text-[#007acc]">IA</span>
              </div>
              <h1 className="text-xl font-bold text-[#DCDCDC] mb-2">Bem-vindo ao IA-Hub</h1>
              <p className="text-[#A3A3A3] text-sm mb-6">
                Multi-Agent IDE Control Plane. Vamos configurar seu ambiente em 3 passos rápidos.
              </p>
              <button
                onClick={() => setStep("import_repo")}
                className="w-full py-2.5 bg-[#007acc] hover:bg-[#007acc]/90 text-white rounded-lg text-sm font-medium transition-colors"
              >
                Começar
              </button>
            </div>
          )}

          {step === "import_repo" && (
            <div>
              <h2 className="text-lg font-bold text-[#DCDCDC] mb-1">Importar Repositório</h2>
              <p className="text-[#A3A3A3] text-sm mb-4">
                Selecione a pasta do projeto que você quer gerenciar com agentes de IA.
              </p>
              <input
                type="text"
                value={repoPath}
                onChange={(e) => setRepoPath(e.target.value)}
                placeholder="C:\Users\you\projects\my-app"
                className="w-full px-3 py-2 bg-[#0B0B0B] border border-[#232323] rounded-lg text-sm text-[#DCDCDC] placeholder-[#555] focus:outline-none focus:border-[#007acc] mb-4 font-mono"
              />
              <div className="flex gap-2">
                <button
                  onClick={() => setStep("welcome")}
                  className="flex-1 py-2 border border-[#232323] text-[#A3A3A3] rounded-lg text-sm hover:text-[#DCDCDC] hover:border-[#333] transition-colors"
                >
                  Voltar
                </button>
                <button
                  onClick={handleImportRepo}
                  disabled={loading || !repoPath.trim()}
                  className="flex-1 py-2 bg-[#007acc] hover:bg-[#007acc]/90 disabled:opacity-40 text-white rounded-lg text-sm font-medium transition-colors"
                >
                  {loading ? "Importando..." : "Importar"}
                </button>
              </div>
            </div>
          )}

          {step === "add_account" && (
            <div>
              <h2 className="text-lg font-bold text-[#DCDCDC] mb-1">Adicionar Conta</h2>
              <p className="text-[#A3A3A3] text-sm mb-4">
                Conecte uma conta de provider de IA (Claude, Antigravity, Codex, etc.)
              </p>
              <select
                value={provider}
                onChange={(e) => setProvider(e.target.value)}
                className="w-full px-3 py-2 bg-[#0B0B0B] border border-[#232323] rounded-lg text-sm text-[#DCDCDC] focus:outline-none focus:border-[#007acc] mb-3"
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
                className="w-full px-3 py-2 bg-[#0B0B0B] border border-[#232323] rounded-lg text-sm text-[#DCDCDC] placeholder-[#555] focus:outline-none focus:border-[#007acc] mb-4"
              />
              <div className="flex gap-2">
                <button
                  onClick={() => setStep("import_repo")}
                  className="flex-1 py-2 border border-[#232323] text-[#A3A3A3] rounded-lg text-sm hover:text-[#DCDCDC] hover:border-[#333] transition-colors"
                >
                  Voltar
                </button>
                <button
                  onClick={handleAddAccount}
                  disabled={loading || !accountLabel.trim()}
                  className="flex-1 py-2 bg-[#007acc] hover:bg-[#007acc]/90 disabled:opacity-40 text-white rounded-lg text-sm font-medium transition-colors"
                >
                  {loading ? "Salvando..." : "Adicionar"}
                </button>
              </div>
            </div>
          )}

          {step === "launch" && (
            <div className="text-center">
              <div className="w-12 h-12 rounded-full bg-[#4ADE80]/15 flex items-center justify-center mx-auto mb-4 border border-[#4ADE80]/20">
                <span className="text-[#4ADE80] text-xl">✓</span>
              </div>
              <h2 className="text-lg font-bold text-[#DCDCDC] mb-2">Tudo Pronto!</h2>
              <p className="text-[#A3A3A3] text-sm mb-6">
                Seu ambiente está configurado. Você pode iniciar sessões de agentes a qualquer momento pelo painel principal.
              </p>
              <button
                onClick={onComplete}
                className="w-full py-2.5 bg-[#007acc] hover:bg-[#007acc]/90 text-white rounded-lg text-sm font-medium transition-colors"
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
              className="text-[#555] text-xs hover:text-[#A3A3A3] transition-colors"
            >
              Pular onboarding →
            </button>
          </div>
        )}
      </div>
    </div>
  );
}