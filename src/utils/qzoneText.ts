export interface QzoneTextPart {
  type: "text" | "mention" | "emoji";
  value: string;
  uin?: string;
}

const qzoneTokenPattern = /@\{uin:([^,}]+),nick:([^,}]+)(?:,[^}]*)?\}|\[em\](e\d+)\[\/em\]/g;

export function parseQzoneText(value?: string): QzoneTextPart[] {
  const text = value?.replace(/^[：:]\s*/, "") || "";
  const parts: QzoneTextPart[] = [];
  let cursor = 0;
  for (const match of text.matchAll(qzoneTokenPattern)) {
    const index = match.index ?? 0;
    if (index > cursor) parts.push({ type: "text", value: text.slice(cursor, index) });
    if (match[3]) parts.push({ type: "emoji", value: match[3] });
    else parts.push({ type: "mention", value: `@${match[2]}`, uin: match[1] });
    cursor = index + match[0].length;
  }
  if (cursor < text.length) parts.push({ type: "text", value: text.slice(cursor) });
  return parts.length ? parts : [{ type: "text", value: "该动态没有文字内容" }];
}
