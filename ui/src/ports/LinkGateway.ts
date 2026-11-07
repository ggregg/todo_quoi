/**
 * Port : ouvrir un lien de note (site web, e-mail) **hors** de l'application.
 *
 * La webview ne doit jamais naviguer elle-même vers un lien : elle quitterait TodoQuoi. Le cœur
 * Rust vérifie le lien (http, https, mailto seulement) et le confie au système.
 */
import type { InjectionKey } from 'vue'

export interface LinkGateway {
  /** Ouvre le lien dans le navigateur ou la messagerie du système. */
  open(url: string): Promise<void>
}

export const linkGatewayKey: InjectionKey<LinkGateway> = Symbol('LinkGateway')
