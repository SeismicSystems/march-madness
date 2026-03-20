import { Routes, Route } from "react-router-dom";

import { Header } from "./components/Header";
import { HomePage } from "./pages/HomePage";
import { LeaderboardPage } from "./pages/LeaderboardPage";
import { BracketViewerPage } from "./pages/BracketViewerPage";
import { GroupsPage } from "./pages/GroupsPage";
import { MirrorBracketPage } from "./pages/MirrorBracketPage";
import { MirrorFinalFourPage } from "./pages/MirrorFinalFourPage";
import { MirrorLeaderboardPage } from "./pages/MirrorLeaderboardPage";
import { PublicGroupsPage } from "./pages/PublicGroupsPage";

export default function App() {
  return (
    <div className="min-h-screen bg-bg-primary">
      <Header />

      <main className="max-w-[1800px] mx-auto px-2 sm:px-4 py-4 sm:py-6">
        <Routes>
          <Route path="/" element={<HomePage />} />
          <Route path="/leaderboard" element={<LeaderboardPage />} />
          <Route
            path="/groups/:slug/leaderboard"
            element={<LeaderboardPage />}
          />
          <Route path="/groups" element={<GroupsPage />} />
          <Route path="/groups/public" element={<PublicGroupsPage />} />
          <Route path="/mirrors/id/:id" element={<MirrorLeaderboardPage />} />
          <Route path="/mirrors/id/:id/ff" element={<MirrorFinalFourPage />} />
          <Route
            path="/mirrors/id/:id/bracket/:entrySlug"
            element={<MirrorBracketPage />}
          />
          <Route path="/bracket/:address" element={<BracketViewerPage />} />
        </Routes>
      </main>
    </div>
  );
}
