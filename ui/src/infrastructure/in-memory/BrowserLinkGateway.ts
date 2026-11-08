/** `LinkGateway` pour `pnpm ui:dev` (simple navigateur) : nouvel onglet, sans accès à cette page. */
import type { LinkGateway } from '../../ports/LinkGateway'

export class BrowserLinkGateway implements LinkGateway {
  async open(url: string): Promise<void> {
    if (!/^(https?:|mailto:)/i.test(url)) throw { code: 'invalid-input', message: `lien non pris en charge : ${url}` }
    window.open(url, '_blank', 'noopener,noreferrer')
  }
}
