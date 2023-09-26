import { File } from "./file-listing";

export type FileType = "file" | "music-file" | "directory";

const SUPPORTED_MUSIC_FILE_EXTENSIONS = [".mp3", ".flac", ".wav", ".ogg"];

export function getFileType(file: File): FileType {
  if (file.children !== undefined) {
    return "directory";
  }
  if (
    file.name &&
    SUPPORTED_MUSIC_FILE_EXTENSIONS.some(
      (extension) => file.name?.toLowerCase().endsWith(extension),
    )
  ) {
    return "music-file";
  }
  return "file";
}
