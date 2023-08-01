export type TreeviewFile = {
  type: "File";
  name: string;
  path: string;
};

export type TreeviewDirectory = {
  type: "Directory";
  name: string;
  path: string;
  children: TreeviewItem[];
  isExpanded: boolean;
};

export type NormalizedTreeviewDirectory = Omit<TreeviewDirectory, "children">;

export type TreeviewItem = TreeviewFile | TreeviewDirectory;

export type NormalizedTreeviewItem = TreeviewFile | NormalizedTreeviewDirectory;

export type TreeviewView = { listing: TreeviewItem[] };

export type TreeviewState = {
  listing: string[];
  items: { [path: string]: NormalizedTreeviewItem };
  directoryChildren: { [path: string]: string[] };
};
