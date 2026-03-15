import { Routes, Route } from "react-router-dom";

import { Header } from "./components/Header";
import { useContract } from "./hooks/useContract";
import { HomePage } from "./pages/HomePage";
import { LeaderboardPage } from "./pages/LeaderboardPage";
import { BracketViewerPage } from "./pages/BracketViewerPage";

export default function App() {
  const contract = useContract();

  return (
    <div className="min-h-screen bg-bg-primary">
      <Header entryCount={contract.entryCount} />

      <main className="max-w-[1800px] mx-auto px-2 sm:px-4 py-4 sm:py-6">
        <Routes>
          <Route path="/" element={<HomePage />} />
          <Route path="/leaderboard" element={<LeaderboardPage />} />
          <Route path="/bracket/:address" element={<BracketViewerPage />} />
        </Routes>
      </main>
    </div>
  );
}
