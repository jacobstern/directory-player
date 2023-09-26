import { createContext } from "react";
import { FileListing } from "../core/file-listing";

export const FileListingContext = createContext<FileListing | null>(null);

export default FileListingContext;
