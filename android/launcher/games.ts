export type Game = {id: string; title: string; year: string; runtime?: 'iw4'};
export const games: readonly Game[] = [
  {id: 'mw2', title: 'Modern Warfare 2', year: '2009', runtime: 'iw4'},
  {id: 'cod4', title: 'Call of Duty 4', year: 'Modern Warfare'},
  {id: 'waw', title: 'World at War', year: '2008'},
  {id: 'bo', title: 'Black Ops', year: '2010'},
  {id: 'mw3', title: 'Modern Warfare 3', year: '2011'},
  {id: 'bo2', title: 'Black Ops II', year: '2012'},
  {id: 'bo3', title: 'Black Ops III', year: '2015'},
];
export type Installation = {
  configured: boolean;
  valid: boolean;
  name?: string;
  error?: string;
  common?: string;
  map?: string;
  runtimeStatus?: string;
};
