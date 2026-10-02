import { createContext, useContext, type ReactNode } from "react";

const OrgCardMotionContext = createContext<number | null>(null);

/** Opts a card into the Overview entrance. Chat cards stay still. */
export function OrgCardMotion({
  children,
  enterIndex,
}: {
  children: ReactNode;
  enterIndex: number;
}) {
  return (
    <OrgCardMotionContext.Provider value={enterIndex}>
      {children}
    </OrgCardMotionContext.Provider>
  );
}

export function useOrgCardMotionIndex(): number | null {
  return useContext(OrgCardMotionContext);
}
