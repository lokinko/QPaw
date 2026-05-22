import { useMemo } from "react";
import { PetWindow } from "./components/PetWindow";
import { SettingsWindow } from "./components/SettingsWindow";

export function App() {
  const view = useMemo(() => new URLSearchParams(window.location.search).get("view"), []);

  return view === "settings" ? <SettingsWindow /> : <PetWindow />;
}
