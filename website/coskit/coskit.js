'use strict';
const range = document.querySelector('#compare-range');
const comparison = document.querySelector('.comparison');
const output = document.querySelector('#compare-value');
range.addEventListener('input', () => {
  const value = Math.min(100, Math.max(0, Number(range.value)));
  comparison.style.setProperty('--split', `${value}%`);
  output.value = `${value}%`;
  range.setAttribute('aria-valuetext', `原片 ${value}%，成片 ${100 - value}%`);
});
document.querySelector('#copy-build').addEventListener('click', async () => {
  const status = document.querySelector('#copy-status');
  try {
    await navigator.clipboard.writeText(document.querySelector('#build-code').textContent.trim());
    status.textContent = '构建命令已复制。';
  } catch {
    status.textContent = '此环境无法写入剪贴板，请手动选择上方命令复制。';
  }
});
document.querySelectorAll('img').forEach(img => img.addEventListener('error', () => {
  img.classList.add('missing');
  img.title = '演示素材单独交付；部署时请将素材包复制到 media/。';
}));
