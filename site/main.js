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
  const linuxAssets = {
    appimage: release.assets.find((a) => a.name.endsWith('_amd64.AppImage')),
    deb: release.assets.find((a) => a.name.endsWith('_amd64.deb')),
    rpm: release.assets.find((a) => a.name.endsWith('.x86_64.rpm')),
  };
  const linux = navigator.userAgent.includes('Linux') && !navigator.userAgent.includes('Android');
  const button = document.getElementById('download');
  const windowsButton = document.getElementById('download-windows');
  const linuxDownloads = document.getElementById('linux-downloads');
  for (const [format, asset] of Object.entries(linuxAssets)) {
    const link = linuxDownloads.querySelector(`[data-linux="${format}"]`);
    if (asset) link.href = asset.browser_download_url;
    else link.remove();
  }
  linuxDownloads.hidden = !Object.values(linuxAssets).some(Boolean);
  if (linux) {
    button.hidden = !linuxAssets.appimage;
    windowsButton.hidden = !installer;
  }
  if (installer) {
    windowsButton.href = installer.browser_download_url;
    if (!linux) {
      button.href = installer.browser_download_url;
      const mb = (installer.size / 1024 / 1024).toFixed(1);
      document.getElementById('download-meta').textContent = `Version ${release.tag_name.replace(/^v/, '')} · ${mb} MB · Windows 10 / 11 x64 · MIT licensed`;
    }
  }
  if (linux && linuxAssets.appimage) {
    button.href = linuxAssets.appimage.browser_download_url;
    button.textContent = 'Download AppImage';
    windowsButton.hidden = !installer;
    document.getElementById('download-meta').textContent = `Version ${release.tag_name.replace(/^v/, '')} · Linux x64 · Free and MIT licensed`;
  }
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
