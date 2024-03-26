/** @type {import('next').NextConfig} */
const nextConfig = {
  webpack: (config, { isServer }) => {
    if (!isServer) {
      config.experiments = {
        ...config.experiments,
        asyncWebAssembly: true,
        syncWebAssembly: true,
        topLevelAwait: true,
      }
    }
    return config;
  }
};

export default nextConfig;
