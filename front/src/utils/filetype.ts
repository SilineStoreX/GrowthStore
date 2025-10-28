/**
 * @description 根据文件类型获得
 * @param {String} key Storage名称
 * @return string
 */
export function get_filetype(extname: string) {
  if (!extname) return "Folder"
  switch(extname.toLowerCase()) {
    case "jpg":
    case "png":
    case "gif": 
    case "jpeg":
    case "bmp":
    case "webp":
      return "PictureFilled"
    case "folder": 
      return "Folder"
    default: 
      return "Document"
  }
}
