// Fills in the download link and release notes from the GitHub Releases API.
// Without JavaScript (or if the API is unreachable) the static fallback links stay in place.
const REPO = 'SaGgaSsa/keepshot';

async function loadReleases() {
  const res = await fetch(`https://api.github.com/repos/${REPO}/releases?per_page=5`, {
    // Ask for GitHub's rendered (and sanitized) HTML of each release body.
    headers: { Accept: 'application/vnd.github.html+json' },
  });
  if (!res.ok) throw new Error(`GitHub API responded ${res.status}`);
  return (await res.json()).filter((r) => !r.draft && !r.prerelease);
}

function showDownload(release) {
  const installer = release.assets.find((a) => a.name.endsWith('_x64-setup.exe'));
  if (!installer) return;
  document.getElementById('download').href = installer.browser_download_url;
  const mb = (installer.size / 1024 / 1024).toFixed(1);
  document.getElementById('download-meta').textContent =
    `Version ${release.tag_name.replace(/^v/, '')} · ${mb} MB · Windows 10 / 11 x64 · MIT licensed`;
}

function showReleases(releases) {
  const list = document.getElementById('release-list');
  const dateFormat = new Intl.DateTimeFormat('en', { dateStyle: 'medium' });
  list.replaceChildren(
    ...releases.slice(0, 3).map((release) => {
      const article = document.createElement('article');
      article.className = 'release';

      const head = document.createElement('header');
      const tag = document.createElement('a');
      tag.className = 'tag';
      tag.href = release.html_url;
      tag.textContent = release.tag_name;
      const date = document.createElement('time');
      date.dateTime = release.published_at;
      date.textContent = dateFormat.format(new Date(release.published_at));
      head.append(tag, date);

      const body = document.createElement('div');
      body.className = 'release-body';
      body.innerHTML = release.body_html ?? '';

      article.append(head, body);
      return article;
    }),
  );
}

// Screenshots are optional: drop figures whose image is missing, and the section if none load.
function pruneScreenshots() {
  const section = document.getElementById('shots');
  const prune = (img) => {
    img.closest('figure').remove();
    if (!section.querySelector('figure')) section.remove();
  };
  for (const img of section.querySelectorAll('img')) {
    if (img.complete && img.naturalWidth === 0) prune(img);
    else img.addEventListener('error', () => prune(img));
  }
}

pruneScreenshots();
loadReleases()
  .then((releases) => {
    if (releases.length === 0) return;
    showDownload(releases[0]);
    showReleases(releases);
  })
  .catch((err) => console.warn('Could not load releases:', err));
