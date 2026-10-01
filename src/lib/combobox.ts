interface SearchableOption {
  value: string;
  label: string;
  description?: string;
}

function normalize(text: string): string {
  return text
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase();
}

/** Keeps options whose label, value, or description contains every word of the query. */
export function filterOptions<T extends SearchableOption>(options: T[], query: string): T[] {
  const words = normalize(query).split(/\s+/).filter(Boolean);
  if (words.length === 0) return options;
  return options.filter((option) => {
    const haystack = normalize(`${option.label} ${option.value} ${option.description ?? ""}`);
    return words.every((word) => haystack.includes(word));
  });
}
