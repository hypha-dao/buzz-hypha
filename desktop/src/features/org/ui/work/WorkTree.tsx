import { Link } from "@tanstack/react-router";

import {
  formatChildrenCounts,
  formatWorkDate,
  type WorkTreeNode,
} from "../../work/model";
import { HolderName } from "./HolderName";
import { StateChip } from "./StateChip";

type WorkTreeProps = {
  nodes: WorkTreeNode[];
};

export function WorkTree({ nodes }: WorkTreeProps) {
  return (
    <ul className="space-y-3" data-testid="org-work-tree">
      {nodes.map((node) => (
        <li key={node.item.id}>
          <WorkTreeRow depth={0} item={node.item} />
          {node.children.length > 0 ? (
            <ul className="mt-1 space-y-1 border-l border-border pl-4">
              {node.children.map((child) => (
                <li key={child.id}>
                  <WorkTreeRow depth={1} item={child} />
                </li>
              ))}
            </ul>
          ) : null}
        </li>
      ))}
    </ul>
  );
}

function WorkTreeRow({
  item,
  depth,
}: {
  item: WorkTreeNode["item"];
  depth: number;
}) {
  const holder = item.state === "offered" ? item.offeredTo : item.dri;
  return (
    <Link
      className="flex items-start justify-between gap-3 rounded-lg px-2 py-2 hover:bg-muted/60 focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-ring"
      data-depth={depth}
      data-testid={`org-work-row-${item.id}`}
      params={{ itemId: item.id }}
      to="/org/work/$itemId"
    >
      <div className="min-w-0">
        <p className="truncate text-sm font-medium">{item.title}</p>
        <p className="mt-0.5 text-2xs text-muted-foreground">
          <HolderName
            pubkey={holder}
            testId={`org-work-row-holder-${item.id}`}
          />
          {item.dueAt !== null ? (
            <>
              {" · "}
              <time data-testid={`org-work-row-due-${item.id}`} dateTime={iso(item.dueAt)}>
                due {formatWorkDate(item.dueAt)}
              </time>
            </>
          ) : null}
        </p>
        {depth === 0 ? (
          <p
            className="mt-1 text-2xs text-muted-foreground"
            data-testid={`org-children-counts-${item.id}`}
          >
            {formatChildrenCounts(item.children)}
          </p>
        ) : null}
      </div>
      <StateChip
        item={item}
        who={
          item.state === "offered" ||
          item.state === "accepted" ||
          item.state === "in_review"
            ? undefined
            : null
        }
      />
    </Link>
  );
}

function iso(unix: number): string {
  return new Date(unix * 1000).toISOString();
}
