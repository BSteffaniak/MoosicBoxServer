import { createHyperChadSite } from 'hyperchad/HyperChadSite';

function getCustomDomain() {
    return {
        name: domainName,
        dns: sst.cloudflare.dns(),
        redirects: [`www.${domainName}`],
    };
}

const isProd = $app.stage === 'prod';
const subdomain = isProd ? '' : `marketing-${$app.stage}.`;
const domain = process.env.DOMAIN ?? 'moosicbox.com';
const domainName = `${subdomain}${domain}`;

const customDomain = getCustomDomain();

const { staticSite } = createHyperChadSite(
    'MoosicBoxMarketingSite',
    'vanillaJs',
    {
        domain: customDomain,
        environment: {},
        errorPage: 'not-found.html',
    },
);

export const outputs = {
    url: staticSite.url,
    host: `https://${domainName}`,
};
