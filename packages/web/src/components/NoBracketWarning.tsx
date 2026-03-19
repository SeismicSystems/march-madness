import { Link } from "react-router-dom";

/**
 * Shown when a connected user tries to join/create a group but hasn't
 * submitted a bracket on-chain yet. Links them back to the homepage
 * where the bracket picker lives.
 */
export function NoBracketWarning() {
  return (
    <div className="rounded-lg bg-yellow-900/20 border border-yellow-700/40 px-4 py-3">
      <p className="text-sm text-yellow-200">
        To join or create a group, you must first{" "}
        <Link
          to="/"
          className="text-accent hover:text-accent-hover underline font-medium"
        >
          submit a bracket
        </Link>
        .
      </p>
    </div>
  );
}
