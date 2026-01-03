export function applyTheme(theme: 'system' | 'dark' | 'light') {
  const root = document.documentElement;
  const isDark = theme === 'dark' || (theme === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches);
  
  if (isDark) {
    root.classList.add('dark');
  } else {
    root.classList.remove('dark');
  }
}

export function initThemeListener(getCurrentTheme: () => 'system' | 'dark' | 'light') {
  window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
    if (getCurrentTheme() === 'system') {
      applyTheme('system');
    }
  });
}
