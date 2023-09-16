import "./file-icon.styles.css";

export type FileIconType = "Directory" | "MusicFile" | "File";

export interface FileIconProps {
  type: FileIconType;
}

export default function FileIcon({ type }: FileIconProps) {
  let iconNode: React.ReactNode;
  switch (type) {
    case "MusicFile":
      iconNode = (
        <svg
          xmlns="http://www.w3.org/2000/svg"
          height="1em"
          viewBox="0 0 512 512"
          fill="currentColor"
        >
          <title>Music File</title>
          {/*! Font Awesome Free 6.4.0 by @fontawesome - https://fontawesome.com License - https://fontawesome.com/license (Commercial License) Copyright 2023 Fonticons, Inc.*/}
          <path d="M499.1 6.3c8.1 6 12.9 15.6 12.9 25.7v72V368c0 44.2-43 80-96 80s-96-35.8-96-80s43-80 96-80c11.2 0 22 1.6 32 4.6V147L192 223.8V432c0 44.2-43 80-96 80s-96-35.8-96-80s43-80 96-80c11.2 0 22 1.6 32 4.6V200 128c0-14.1 9.3-26.6 22.8-30.7l320-96c9.7-2.9 20.2-1.1 28.3 5z" />
        </svg>
      );
      break;
    case "File":
      iconNode = (
        <svg
          xmlns="http://www.w3.org/2000/svg"
          height="1em"
          viewBox="0 0 384 512"
          fill="currentColor"
        >
          <title>File</title>
          {/*! Font Awesome Free 6.4.0 by @fontawesome - https://fontawesome.com License - https://fontawesome.com/license (Commercial License) Copyright 2023 Fonticons, Inc.*/}
          <path d="M320 464c8.8 0 16-7.2 16-16V160H256c-17.7 0-32-14.3-32-32V48H64c-8.8 0-16 7.2-16 16V448c0 8.8 7.2 16 16 16H320zM0 64C0 28.7 28.7 0 64 0H229.5c17 0 33.3 6.7 45.3 18.7l90.5 90.5c12 12 18.7 28.3 18.7 45.3V448c0 35.3-28.7 64-64 64H64c-35.3 0-64-28.7-64-64V64z" />
        </svg>
      );
      break;
    default:
      iconNode = (
        <svg
          xmlns="http://www.w3.org/2000/svg"
          height="1em"
          viewBox="0 0 512 512"
          fill="currentColor"
          stroke="currentColor"
        >
          <title>Folder</title>
          {/*! Font Awesome Free 6.4.0 by @fontawesome - https://fontawesome.com License - https://fontawesome.com/license (Commercial License) Copyright 2023 Fonticons, Inc.*/}
          <path d="M0 96C0 60.7 28.7 32 64 32H196.1c19.1 0 37.4 7.6 50.9 21.1L289.9 96H448c35.3 0 64 28.7 64 64V416c0 35.3-28.7 64-64 64H64c-35.3 0-64-28.7-64-64V96zM64 80c-8.8 0-16 7.2-16 16V416c0 8.8 7.2 16 16 16H448c8.8 0 16-7.2 16-16V160c0-8.8-7.2-16-16-16H286.6c-10.6 0-20.8-4.2-28.3-11.7L213.1 87c-4.5-4.5-10.6-7-17-7H64z" />
        </svg>
      );
  }
  return <i className="file-icon">{iconNode}</i>;
}
