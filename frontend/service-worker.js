let APP_VERSION = "1.0.0";

const CACHE_PREFIX = "SNAKE_CACHE_";
const PWA_PREFIX = "SNAKE_PWA_CACHE_";

const SHELL_FILES = [
  "/",
  "/index.html",
  "/service-worker.js",
  "/manifest.json",
  "/favicon.png",
  "/favicon.svg",
];

const isManagedCache = (name) =>
  typeof name === "string" &&
  (name.startsWith(CACHE_PREFIX) || name.startsWith(PWA_PREFIX));

const cachePrefixLen = (name) => {
  if (name.startsWith(CACHE_PREFIX)) return CACHE_PREFIX.length;
  if (name.startsWith(PWA_PREFIX)) return PWA_PREFIX.length;
  return -1;
};

const safeJson = async (response) => {
  try {
    return await response.json();
  } catch (_) {
    return null;
  }
};

const fetchConfig = async () => {
  try {
    const res = await fetch("/api/config", { cache: "no-store" });
    if (!res.ok) return null;
    return await safeJson(res);
  } catch (err) {
    console.error("Snake SW /api/config fetch failed:", err);
    return null;
  }
};

const getAppVersion = async () => {
  const cfg = await fetchConfig();
  if (cfg && typeof cfg.version === "string" && cfg.version.length > 0) {
    APP_VERSION = cfg.version;
  }
  console.log(`Snake SW app version: ${APP_VERSION}`);
  return APP_VERSION;
};

const getCurrentCacheVersion = async () => {
  const names = await caches.keys();
  const versions = names
    .filter(isManagedCache)
    .map((n) => {
      const len = cachePrefixLen(n);
      return len > 0 ? n.slice(len) : null;
    })
    .filter(Boolean);
  if (versions.length === 0) return null;
  versions.sort();
  return versions[versions.length - 1];
};

const dedupe = (list) => Array.from(new Set(list));

const installNewCache = async (version) => {
  const cacheName = `${CACHE_PREFIX}${version}`;
  console.log(`Snake SW installing cache ${cacheName}`);
  const manifestRes = await fetch("/asset-manifest.json", { cache: "no-store" });
  if (!manifestRes.ok) {
    throw new Error(`asset-manifest.json returned ${manifestRes.status}`);
  }
  const assets = (await safeJson(manifestRes)) || [];
  const assetsToCache = dedupe([...SHELL_FILES, ...assets]);
  const cache = await caches.open(cacheName);
  await cache.addAll(assetsToCache);
  console.log(`Snake cache pre-populated for v${version}`);
};

const cleanupOldCaches = async (currentVersion) => {
  const currentName = `${CACHE_PREFIX}${currentVersion}`;
  const names = await caches.keys();
  const deletes = names
    .filter((name) => isManagedCache(name) && name !== currentName)
    .map((name) => {
      console.log(`Snake cache deleted (old): ${name}`);
      return caches.delete(name);
    });
  return Promise.all(deletes);
};

const checkAndUpdateCache = async () => {
  const appVersion = await getAppVersion();
  const cacheVersion = await getCurrentCacheVersion();
  console.log(`Snake SW check: app=${appVersion} cache=${cacheVersion ?? "none"}`);
  if (!cacheVersion) {
    await installNewCache(appVersion);
    return { updated: true, firstInstall: true };
  }
  if (cacheVersion !== appVersion) {
    await installNewCache(appVersion);
    await cleanupOldCaches(appVersion);
    return { updated: true, firstInstall: false };
  }
  return { updated: false, firstInstall: false };
};

self.addEventListener("install", (event) => {
  console.log(`Snake SW installing v${APP_VERSION}`);
  event.waitUntil(
    (async () => {
      try {
        await getAppVersion();
        await installNewCache(APP_VERSION);
      } catch (err) {
        console.error("Snake SW install failed:", err);
        throw err;
      }
    })().then(() => self.skipWaiting())
  );
});

self.addEventListener("activate", (event) => {
  console.log(`Snake SW activating v${APP_VERSION}`);
  event.waitUntil(
    (async () => {
      const { updated, firstInstall } = await checkAndUpdateCache();
      await self.clients.claim();
      if (!updated) return;
      const clients = await self.clients.matchAll({ includeUncontrolled: true });
      for (const client of clients) {
        client.postMessage({
          type: firstInstall ? "CACHE_INSTALLED" : "CACHE_UPDATED",
          reload: !firstInstall,
          version: APP_VERSION,
        });
      }
    })()
  );
});

self.addEventListener("fetch", (event) => {
  const req = event.request;
  if (req.method !== "GET") return;
  let url;
  try {
    url = new URL(req.url);
  } catch (_) {
    return;
  }
  if (url.origin !== self.location.origin) return;
  if (url.pathname.startsWith("/api/")) return;
  if (url.pathname === "/service-worker.js") return;
  event.respondWith(
    caches.match(req).then((cached) => cached || fetch(req))
  );
});

self.addEventListener("message", (event) => {
  if (!event.data || event.data.type !== "CHECK_VERSION") return;
  checkAndUpdateCache().then(({ updated, firstInstall }) => {
    if (event.ports && event.ports[0]) {
      event.ports[0].postMessage({ updated, firstInstall, version: APP_VERSION });
    }
  });
});
