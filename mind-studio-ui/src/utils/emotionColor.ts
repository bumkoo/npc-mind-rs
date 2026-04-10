const colors: Record<string, string> = {
  Joy: '#66bb6a',
  Distress: '#ef5350',
  Hope: '#4ecdc4',
  Fear: '#ff8a65',
  Satisfaction: '#aed581',
  Disappointment: '#e57373',
  Relief: '#81d4fa',
  FearsConfirmed: '#ff7043',
  HappyFor: '#a5d6a7',
  Resentment: '#ef9a9a',
  Gloating: '#ce93d8',
  Pity: '#90caf9',
  Pride: '#ffd54f',
  Shame: '#bcaaa4',
  Admiration: '#80cbc4',
  Reproach: '#f48fb1',
  Love: '#f06292',
  Hate: '#b71c1c',
  Anger: '#d32f2f',
  Gratitude: '#66bb6a',
  Gratification: '#ffb74d',
  Remorse: '#9e9e9e',
}

export function emotionColor(type: string): string {
  return colors[type] || 'var(--accent)'
}
