const response = await fetch(
  'https://api.github.com/repos/EzyGang/actavoces/releases/latest',
  {
    headers: {
      Accept: 'application/vnd.github.v3+json',
      'User-Agent': 'actavoces-landing-builder'
    }
  }
);

if (!response.ok) {
  console.error('Failed to fetch latest release:', response.status, response.statusText);
  process.exit(1);
}

const release = await response.json();
const tag = release.tag_name;

console.log(tag);
