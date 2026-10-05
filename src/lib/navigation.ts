export interface CardRect { left: number; top: number; width: number; height: number }
export type Direction = 'ArrowLeft' | 'ArrowRight' | 'ArrowUp' | 'ArrowDown';

// Navigation follows rendered card centers, including different column counts.
export function adjacentCard(cards: CardRect[], selected: number, direction: Direction): number {
  const current = cards[selected];
  if (!current) return 0;
  const cx = current.left + current.width / 2;
  const cy = current.top + current.height / 2;
  const horizontal = direction === 'ArrowLeft' || direction === 'ArrowRight';
  const sign = direction === 'ArrowLeft' || direction === 'ArrowUp' ? -1 : 1;
  const candidates = cards.map((card, index) => {
    const dx = card.left + card.width / 2 - cx;
    const dy = card.top + card.height / 2 - cy;
    return { index, dx, dy };
  }).filter(c => c.index !== selected && (horizontal
    ? Math.abs(c.dy) < 2 && c.dx * sign > 1
    : c.dy * sign > 2));
  candidates.sort((a, b) => horizontal ? Math.abs(a.dx) - Math.abs(b.dx)
    : Math.abs(a.dy) - Math.abs(b.dy) || Math.abs(a.dx) - Math.abs(b.dx));
  return candidates[0]?.index ?? selected;
}
